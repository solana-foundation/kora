/**
 * Throws an exception unless {@link SubtleCrypto#digest | `crypto.subtle.digest()`} is available in
 * the current JavaScript environment.
 */
export declare function assertDigestCapabilityIsAvailable(): void;
/**
 * Throws an exception unless {@link SubtleCrypto#generateKey | `crypto.subtle.generateKey()`} is
 * available in the current JavaScript environment and has support for the Ed25519 curve.
 */
export declare function assertKeyGenerationIsAvailable(): Promise<void>;
/**
 * Throws an exception unless {@link SubtleCrypto#exportKey | `crypto.subtle.exportKey()`} is
 * available in the current JavaScript environment.
 */
export declare function assertKeyExporterIsAvailable(): void;
/**
 * Throws an exception unless {@link SubtleCrypto#sign | `crypto.subtle.sign()`} is available in the
 * current JavaScript environment.
 */
export declare function assertSigningCapabilityIsAvailable(): void;
/**
 * Throws an exception unless {@link SubtleCrypto#verify | `crypto.subtle.verify()`} is available in
 * the current JavaScript environment.
 */
export declare function assertVerificationCapabilityIsAvailable(): void;
//# sourceMappingURL=subtle-crypto.d.ts.map