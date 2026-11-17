import { afterEach } from "bun:test";
import { cleanup } from "@testing-library/react";

// React avisa de las actualizaciones fuera de `act()` salvo que sepa que corre en un test.
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

afterEach(cleanup);
