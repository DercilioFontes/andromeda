// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Test framework globals
declare global {
  function describe(name: string, fn: () => void): void;
  function it(name: string, fn: () => void): void;
  function expect(value: any): Expectation;
}

interface Expectation {
  toBe(expected: any): void;
  toEqual(expected: any): void;
  toBeTruthy(): void;
  toBeFalsy(): void;
  toBeNull(): void;
  toBeUndefined(): void;
  toBeDefined(): void;
  toThrow(): void;
  not: Expectation;
}

class ExpectationImpl implements Expectation {
  private value: any;
  private negated: boolean = false;

  constructor(value: any) {
    this.value = value;
  }

  get not(): Expectation {
    this.negated = !this.negated;
    return this;
  }

  private check(condition: boolean, message: string): void {
    if (this.negated ? condition : !condition) {
      throw new Error(message);
    }
  }

  toBe(expected: any): void {
    this.check(
      this.value === expected,
      `Expected ${this.value} ${this.negated ? "not " : ""}to be ${expected}`,
    );
  }

  toEqual(expected: any): void {
    this.check(
      deepEqual(this.value, expected),
      `Expected ${JSON.stringify(this.value)} ${this.negated ? "not " : ""}to equal ${JSON.stringify(expected)}`,
    );
  }

  toBeTruthy(): void {
    this.check(
      !!this.value,
      `Expected ${this.value} ${this.negated ? "not " : ""}to be truthy`,
    );
  }

  toBeFalsy(): void {
    this.check(
      !this.value,
      `Expected ${this.value} ${this.negated ? "not " : ""}to be falsy`,
    );
  }

  toBeNull(): void {
    this.check(
      this.value === null,
      `Expected ${this.value} ${this.negated ? "not " : ""}to be null`,
    );
  }

  toBeUndefined(): void {
    this.check(
      this.value === undefined,
      `Expected ${this.value} ${this.negated ? "not " : ""}to be undefined`,
    );
  }

  toBeDefined(): void {
    this.check(
      this.value !== undefined,
      `Expected ${this.value} ${this.negated ? "not " : ""}to be defined`,
    );
  }

  toThrow(): void {
    let threw = false;
    try {
      if (typeof this.value === "function") {
        this.value();
      }
    } catch (e) {
      threw = true;
    }
    this.check(
      threw,
      `Expected function ${this.negated ? "not " : ""}to throw`,
    );
  }
}

function deepEqual(a: any, b: any): boolean {
  if (a === b) return true;
  if (a == null || b == null) return a === b;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!deepEqual(a[i], b[i])) return false;
    }
    return true;
  }
  if (typeof a === "object" && typeof b === "object") {
    const keysA = Object.keys(a);
    const keysB = Object.keys(b);
    if (keysA.length !== keysB.length) return false;
    for (const key of keysA) {
      if (!keysB.includes(key)) return false;
      if (!deepEqual(a[key], b[key])) return false;
    }
    return true;
  }
  return false;
}

// Global test functions
(globalThis as any).describe = function (name: string, fn: () => void) {
  (globalThis as any).__andromeda_test_describe(name, fn);
  fn(); // Execute the test suite
};

(globalThis as any).it = function (name: string, fn: () => void) {
  try {
    fn();
    (globalThis as any).__andromeda_test_it_passed(name);
  } catch (e) {
    (globalThis as any).__andromeda_test_it_failed(name, (e as Error).message);
  }
};

(globalThis as any).expect = function (value: any): Expectation {
  return new ExpectationImpl(value);
};

// Export for module usage
export { describe, it, expect };
