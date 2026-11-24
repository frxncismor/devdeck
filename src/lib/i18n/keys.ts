import type { messages } from "./index";

type Json = { readonly [k: string]: string | Json };

type Flatten<T extends Json, P extends string = ""> = {
  [K in keyof T & string]: T[K] extends string
    ? `${P}${K}`
    : T[K] extends Json
      ? Flatten<T[K], `${P}${K}.`>
      : never;
}[keyof T & string];

/** Plural keys are stored as `<key>_one` / `<key>_other` and used as `<key>`. */
type StripPlural<K extends string> = K extends `${infer B}_${"one" | "other"}`
  ? B
  : K;

/** Every valid translation key, derived from the `en` catalogue. */
export type MessageKey = StripPlural<Flatten<(typeof messages)["en"]>>;
