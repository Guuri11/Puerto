# application/

Empty on purpose. The generated screens call the repository port directly
(`useDependencies().productRepository.list()`): a frontend CRUD use case would only delegate, the
server owns validation.

Write a use case here when there is logic to hold — a rule, or an orchestration of two
repositories — as `application/<entity>/<action>.ts`, taking its ports as arguments. Then the
container calls the use case instead of the repository. It may import `domain/` only.
