use std::{fs, path::Path};

/// Add a new SQLx migration file: `infrastructure/migrations/{version}_{name}.sql`.
///
/// The file is written directly, in the format `sqlx migrate add` uses, rather than by shelling
/// out to it. sqlx keys migrations by the 14-digit timestamp prefix and `sqlx migrate add` only
/// has one-second resolution, so two scaffolds within the same second produced the same version
/// and `sqlx migrate run` refused with "previously applied but has been modified". The version
/// here is the current UTC time, bumped past the newest existing migration when needed.
///
/// `content` overrides the body written into the migration file. Pass `None` to leave
/// a generic placeholder (used by `puerto generate migration`). Pass `Some(sql)` to
/// write a pre-filled schema (used by `puerto generate scaffold` in a `--db` project).
pub fn run_migration(
    name: &str,
    base: &Path,
    content: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let migrations_dir = base.join("infrastructure/migrations");
    fs::create_dir_all(&migrations_dir)?;

    // Normalise name: spaces → underscores, lowercase.
    let normalised = name.replace(' ', "_").to_lowercase();

    let version = next_version(&migrations_dir, utc_timestamp_now())?;
    let path = migrations_dir.join(format!("{version}_{normalised}.sql"));
    let header = format!(
        "-- Puerto migration: {normalised}\n-- Run `make sqlx/prepare` after editing this file.\n\n"
    );
    let body = content.unwrap_or("-- Add migration script here\n");
    fs::write(&path, format!("{header}{body}"))?;

    println!("✓ Migration '{normalised}' created in infrastructure/migrations/");
    // The project builds with SQLX_OFFLINE=true: until the query cache knows the new schema, the
    // queries that use it do not compile.
    if content.is_some() {
        println!("  Schema pre-filled — run: make sqlx/migrate && make sqlx/prepare");
    } else {
        println!("  Edit the file, then run: make sqlx/migrate && make sqlx/prepare");
    }

    Ok(())
}

/// Writes `create_{snake}_table` unless a migration of that name already exists — a forced
/// re-scaffold or `generate repository` on a scaffolded entity wrote a second `CREATE TABLE`, and
/// `sqlx migrate run` failed on the existing table. A changed field set is the user's migration
/// to write: the existing one may already be applied somewhere.
pub fn run_create_table_migration(
    snake: &str,
    base: &Path,
    sql: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = format!("create_{snake}_table");
    let suffix = format!("_{name}.sql");
    let existing = fs::read_dir(base.join("infrastructure/migrations"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|file| file.ends_with(&suffix));
    let Some(existing) = existing else {
        return run_migration(&name, base, Some(sql));
    };
    println!(
        "• infrastructure/migrations/{existing} already creates the table — no new migration."
    );
    println!("  If the fields changed: not applied anywhere yet → edit that file;");
    println!("  applied → `puerto generate migration <name>` with the ALTER TABLE.");
    println!("  Then: make sqlx/migrate && make sqlx/prepare");
    Ok(())
}

/// CLI entry point for `puerto generate migration <name>`: only meaningful in a `--db` project.
pub fn run_generate_migration(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    if !config.project.db {
        return Err(
            "this project has no database — migrations need a project created with \
                    `puerto new --db`"
                .into(),
        );
    }
    run_migration(name, base, None)
}

/// The version for a new migration: `now`, or one past the newest existing version if that is
/// not already earlier than `now`.
fn next_version(migrations_dir: &Path, now: u64) -> Result<u64, std::io::Error> {
    let newest = fs::read_dir(migrations_dir)?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let prefix = name.to_string_lossy().split('_').next()?.to_string();
            prefix.parse::<u64>().ok()
        })
        .max();
    Ok(match newest {
        Some(v) if v >= now => bump_timestamp(v),
        _ => now,
    })
}

/// `YYYYMMDDHHMMSS` + 1 second, carried through minutes, hours and days.
fn bump_timestamp(v: u64) -> u64 {
    let (date, time) = (v / 1_000_000, v % 1_000_000);
    let secs = (time / 10_000) * 3600 + (time / 100 % 100) * 60 + time % 100 + 1;
    let (y, m, d) = (date / 10_000, date / 100 % 100, date % 100);
    let days = days_from_civil(y as i64, m as u32, d as u32) + (secs / 86_400) as i64;
    let secs = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format_timestamp(y, m, d, secs)
}

fn utc_timestamp_now() -> u64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    timestamp_from_unix_secs(secs)
}

/// Seconds since the Unix epoch → `YYYYMMDDHHMMSS` (UTC).
fn timestamp_from_unix_secs(secs: u64) -> u64 {
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format_timestamp(y, m, d, secs % 86_400)
}

fn format_timestamp(y: i64, m: u32, d: u32, secs_of_day: u64) -> u64 {
    let (h, min, s) = (secs_of_day / 3600, secs_of_day / 60 % 60, secs_of_day % 60);
    format!("{y:04}{m:02}{d:02}{h:02}{min:02}{s:02}")
        .parse()
        .unwrap_or(0)
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 → (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Inverse of [`civil_from_days`].
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_carries_across_minute_hour_and_day() {
        assert_eq!(bump_timestamp(20261010065431), 20261010065432);
        assert_eq!(bump_timestamp(20261010065459), 20261010065500);
        assert_eq!(bump_timestamp(20261231235959), 20270101000000);
        assert_eq!(bump_timestamp(20240228235959), 20240229000000);
    }

    #[test]
    fn civil_round_trips() {
        for days in [-1, 0, 1, 19_000, 20_736] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn should_use_the_current_time_when_existing_migrations_are_older() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("20200101000000_old.sql"), "").unwrap();

        // Act
        let version = next_version(dir.path(), 20261010065431).unwrap();

        // Assert
        assert_eq!(version, 20261010065431);
    }

    #[test]
    fn should_bump_past_a_migration_created_in_the_same_second() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("20261010065431_first.sql"), "").unwrap();

        // Act
        let version = next_version(dir.path(), 20261010065431).unwrap();

        // Assert
        assert_eq!(version, 20261010065432);
    }

    #[test]
    fn should_map_known_days_to_their_calendar_dates() {
        assert_eq!(civil_from_days(20_736), (2026, 10, 10));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(days_from_civil(2026, 10, 10), 20_736);
    }

    #[test]
    fn should_format_now_as_a_plausible_utc_timestamp() {
        // Act
        let now = utc_timestamp_now();

        // Assert
        let (date, time) = (now / 1_000_000, now % 1_000_000);
        assert!((20260101..21000101).contains(&date), "{now}");
        assert!(
            time / 10_000 < 24 && time / 100 % 100 < 60 && time % 100 < 60,
            "{now}"
        );
    }

    #[test]
    fn should_convert_unix_seconds_to_a_utc_timestamp() {
        assert_eq!(timestamp_from_unix_secs(0), 19700101000000);
        assert_eq!(timestamp_from_unix_secs(1_791_615_271), 20261010065431);
    }

    #[test]
    fn should_map_era_boundaries_to_their_calendar_dates() {
        // The 400-year era starts on March 1st; leap days and century years are where the
        // `/ 1460`, `/ 36_524` and `/ 146_096` corrections in `civil_from_days` matter.
        for (days, date) in [
            (-25_508, (1900, 3, 1)),
            (-1, (1969, 12, 31)),
            (11_017, (2000, 3, 1)),
            (47_540, (2100, 2, 28)),
            (47_541, (2100, 3, 1)),
            (157_113, (2400, 2, 29)),
        ] {
            assert_eq!(civil_from_days(days), date, "day {days}");
            assert_eq!(days_from_civil(date.0, date.1, date.2), days, "{date:?}");
        }
    }
}
