import { Option } from './option';
/**
 * Unwraps the value of an {@link Option}, returning its contained value or a fallback.
 *
 * This function extracts the value `T` from an `Option<T>` type.
 * - If the option is {@link Some}, it returns the contained value `T`.
 * - If the option is {@link None}, it returns the fallback value `U`, which defaults to `null`.
 *
 * @typeParam T - The type of the contained value.
 * @typeParam U - The type of the fallback value (defaults to `null`).
 *
 * @param option - The {@link Option} to unwrap.
 * @param fallback - A function that provides a fallback value if the option is {@link None}.
 * @returns The contained value if {@link Some}, otherwise the fallback value.
 *
 * @example
 * Unwrapping an `Option` with no fallback.
 * ```ts
 * unwrapOption(some('Hello World')); // "Hello World"
 * unwrapOption(none());              // null
 * ```
 *
 * @example
 * Providing a custom fallback value.
 * ```ts
 * unwrapOption(some('Hello World'), () => 'Default'); // "Hello World"
 * unwrapOption(none(), () => 'Default');              // "Default"
 * ```
 *
 * @see {@link Option}
 * @see {@link Some}
 * @see {@link None}
 */
export declare function unwrapOption<T>(option: Option<T>): T | null;
export declare function unwrapOption<T, U>(option: Option<T>, fallback: () => U): T | U;
/**
 * Wraps a nullable value into an {@link Option}.
 *
 * - If the input value is `null`, this function returns {@link None}.
 * - Otherwise, it wraps the value in {@link Some}.
 *
 * @typeParam T - The type of the contained value.
 *
 * @param nullable - The nullable value to wrap.
 * @returns An {@link Option} wrapping the value.
 *
 * @example
 * Wrapping nullable values.
 * ```ts
 * wrapNullable('Hello World'); // Option<string> (Some)
 * wrapNullable<string>(null);  // Option<string> (None)
 * ```
 *
 * @see {@link Option}
 * @see {@link Some}
 * @see {@link None}
 */
export declare const wrapNullable: <T>(nullable: T | null) => Option<T>;
//# sourceMappingURL=unwrap-option.d.ts.map