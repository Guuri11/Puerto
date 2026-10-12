# 0020 — Mail: a port per mail, templates in infrastructure, sent from the worker

**Status:** Accepted (2026-10-12). Spec: `PLAN.md` § Phase 14. Builds on ADR-0017 (runtime crates:
ports in core, adapters in their own crate) and ADR-0018 (events, the worker).

## Context

Sending an email is the most common reaction to something that happened: a welcome mail when a
customer signs up, a receipt when an order is paid. The frameworks Puerto is compared with agree
on the shape:

| | A mail is | Subject | Templates | Sending | Dev / tests |
|---|---|---|---|---|---|
| Rails (ActionMailer) | a method on a mailer class | in code | `app/views/<mailer>/<mail>.{html,text}.erb` | `deliver_now` / `deliver_later` (a job) | `:test` delivery, browser previews |
| Laravel (Mailable) | a class, `envelope()` + `content()` | in code | `resources/views/mail/…` | `send()` / `queue()` | `MAIL_MAILER=log/array`, `Mail::fake()` |
| Phoenix (Swoosh) | a function building an `Email` | in code | none imposed | `Mailer.deliver()` | `Local` adapter + mailbox preview, `Test` adapter |
| Loco (Rust) | a function on a mailer struct | **a template file** | **one folder per mail**: `subject.t`, `html.t`, `text.t`, embedded with `include_dir!` | enqueued to a background worker, or `deliver_now` | SMTP to MailCatcher, a stub in tests |

Common to all of them: a client configured once from the environment and usable anywhere; each
mail is the user's code, taking typed data and deciding the recipient; templates sit in a folder
per mail; the caller chooses between sending now and sending later; tests use a fake that records.
None of them declares mails in a config file. An earlier draft of this ADR did (`[[mail]] { on,
to }` regenerated into a consumer), and it was dropped: it took away exactly the freedom these
frameworks leave to the user.

## Decision

### 1. One port per mail in the domain, its implementation in infrastructure

`puerto generate mail welcome` writes, once, code the user owns from then on:

- `business/src/domain/mail/welcome.rs`: `WelcomeMail` (the data: `to` plus primitive fields) and
  the port `WelcomeMailer { async fn send(&self, mail: &WelcomeMail) -> Result<(), MailError> }`
  with its `mockall` mock. The domain stays free of template engines and SMTP.
- `infrastructure/src/mail/welcome.rs`: `TemplateWelcomeMailer`, which renders the templates and
  hands the `Email` to the client. Its tests render the templates from sample data, so a typo
  fails `make test`, not a delivery.
- `infrastructure/mail/welcome/subject.txt`, `body.html`, `body.txt`: the templates, embedded with
  `include_str!` and rendered with `minijinja` in strict mode (an unknown variable is an error).
  The subject is a template next to the body, as in Loco: one mail's text lives in one folder.

A typed port per mail, rather than one generic `send_template(name, json)` (Loco's shape), is what
Puerto does everywhere else: callers depend on a port they can mock, and a missing variable is a
compile error instead of a render error at send time.

Embedding costs a rebuild to change a mail's text. A container image ships its templates anyway,
so reading them at runtime would save no deploy, and it would move a broken template from
`make test` to production.

### 2. The client: ports in `puerto-core`, SMTP in `puerto-mail`

- `puerto_core::mail`: `Email { to, subject, html, text }`, the `Mailer` port, `MailError::
  {Transient, Permanent}`, and the `SentMails` port (`was_sent` / `mark_sent` by event id and
  mail name).
- `puerto-mail` (new runtime crate, `lettre` + `minijinja`): `SmtpMailer` with
  `MailConfig::from_env` (`SMTP_URL`, `MAIL_FROM`), `Templates` (the renderer), `send_once` (the
  idempotent send a consumer calls, § 3) and `RecordingMailer` for tests.
- `puerto-sqlx::mail::PgSentMails` over `sent_mails (event_id, mail, sent_at)`, primary key
  `(event_id, mail)`.

Every project can generate mails. A project without `--db` has no `docker-compose.yml`, so the
first `generate mail` creates one holding only Mailpit.

### 3. v1 sends from the worker only

`puerto generate mail welcome --on customer_created --to email` (an `events` project) also writes
`worker/src/consumers/mail_welcome.rs`: a consumer the user owns, which builds `WelcomeMail` from the
snapshot and calls `send_once`. `[[mail]] { name, on }` in puerto.toml lists it, so `registry.rs`
can wire it as the consumer `mail_welcome`, the same way it wires a `[[consumer]]`. The toml
entry holds only the wiring, never the mail's content.

Sending from a use case (Rails' `deliver_now`) is left out of v1. `bootstrap.rs` is regenerated,
so a mailer field added by hand to a `…UseCaseImpl` would stop it compiling, and wiring one in
needs its own design. Until then a mail generated in a project without events has its port,
implementation and tests, but nothing sends it.

### 4. Delivery: retry what may pass, dead-letter what never will, prefer a duplicate to a loss

An SMTP send cannot share a transaction with the delivery row. `send_once` checks `was_sent`,
sends, and then calls `mark_sent`. A crash between the send and the mark sends the mail again on
redelivery. Marking first would lose it instead, and a lost receipt is worse than a second one.

| What failed | Outcome |
|---|---|
| Payload does not decode into the snapshot | dead letter `payload_invalid` (the stub's decode) |
| A template does not render | dead letter `permanent_error` |
| The recipient is empty | `Ack`, logged: there is nobody to send to |
| SMTP 5xx (`lettre`'s `is_permanent`) | dead letter `permanent_error` |
| SMTP 4xx, connection refused, timeout, TLS | `Retry` (the delivery policy's backoff, then dead letter) |
| `sent_mails` unreadable before the send | `Retry` |
| `sent_mails` not written after the send | `Ack`, logged as an error: a retry would send it again |

### 5. Development runs against Mailpit

`docker-compose.yml` gains a `mailpit` service (SMTP on 1025, UI and API on 8025), and `.env`
gets `SMTP_URL=smtp://localhost:1025` and `MAIL_FROM=no-reply@<project>.localhost`. Every mail
sent in development lands in the UI (`make mail/ui`), and none reaches a real inbox. Production
points `SMTP_URL` at its provider (`smtps://user:pass@smtp.example.com`). The end-to-end test reads
Mailpit's API (`GET /api/v1/messages`). Mailpit is used over MailCatcher because it is actively
maintained and has a smaller image and a documented API (MailCatcher's last release, v0.10.0, is
from 2024).

## Alternatives rejected

- **`[[mail]] { on, to }` regenerated into a consumer.** No code to write, but no room to change
  anything either: a computed recipient, a condition, extra data. No framework compared does it.
- **One generic `Mailer::send_template(name, to, json)` port.** It is simpler, but untyped, and a
  missing variable shows only at send time.
- **A `log` mailer for development.** It shows that a mail would go out but not what it looks
  like, and it is a code path production never runs. Mailpit runs the real SMTP adapter.
- **Templates read at runtime.** See § 1.

## Consequences

- One more runtime crate, `puerto-mail`, released in lockstep after `puerto-events`.
- `sent_mails` grows by one row per mail sent. Pruning it is the project's job.
- Sending from a use case is the next decision. It also settles how any extra dependency of a
  use case gets wired, which is the same `bootstrap.rs` problem.
