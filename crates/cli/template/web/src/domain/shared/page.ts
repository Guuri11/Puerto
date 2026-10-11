/** Which slice of a list to read. The API caps `limit`; the page it answers says what it applied. */
export interface PageRequest {
  limit: number;
  offset: number;
}

/** One page of a list, and how many items the whole list has. */
export interface Page<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}

/** The page size the list screens ask for. */
export const PAGE_SIZE = 20;
