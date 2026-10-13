import { afterEach } from "bun:test";
import { cleanup } from "@testing-library/react";

// React warns about updates outside `act()` unless it knows it runs under a test.
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

afterEach(cleanup);
