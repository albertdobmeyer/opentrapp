import { initialArgValues, nonEmpty, type ArgLike } from "./helpers";

const arg = (over: Partial<ArgLike>): ArgLike => ({
  id: "x",
  name: "X",
  type: "string",
  required: false,
  options: [],
  ...over,
});

describe("component-detail helpers", () => {
  describe("initialArgValues", () => {
    test("seeds from primitive defaults; empty for missing or non-primitive", () => {
      const args: ArgLike[] = [
        arg({ id: "a", default: "hello" }),
        arg({ id: "b", type: "number", default: 5 }),
        arg({ id: "c", type: "boolean", default: true }),
        arg({ id: "d" }), // no default
        arg({ id: "e", default: { nested: 1 } }), // non-primitive → ""
        arg({ id: "f", default: ["arr"] }), // non-primitive → ""
      ];
      expect(initialArgValues(args)).toEqual({
        a: "hello",
        b: "5",
        c: "true",
        d: "",
        e: "",
        f: "",
      });
    });

    test("handles falsy primitive defaults without dropping them", () => {
      const args: ArgLike[] = [
        arg({ id: "zero", type: "number", default: 0 }),
        arg({ id: "false", type: "boolean", default: false }),
        arg({ id: "empty", default: "" }),
      ];
      expect(initialArgValues(args)).toEqual({ zero: "0", false: "false", empty: "" });
    });

    test("empty arg list → empty map", () => {
      expect(initialArgValues([])).toEqual({});
    });
  });

  describe("nonEmpty", () => {
    test("drops empty-string values, keeps the rest (including '0')", () => {
      expect(nonEmpty({ a: "x", b: "", c: "0", d: "false" })).toEqual({
        a: "x",
        c: "0",
        d: "false",
      });
    });

    test("all-empty → empty map", () => {
      expect(nonEmpty({ a: "", b: "" })).toEqual({});
    });

    test("empty input → empty map", () => {
      expect(nonEmpty({})).toEqual({});
    });
  });
});
