/**
 * Quick primer on bitwise operations: https://stackoverflow.com/a/1436448/802047
 */
export declare enum AccountRole {
    WRITABLE_SIGNER = 3,// prettier-ignore
    READONLY_SIGNER = 2,// prettier-ignore
    WRITABLE = 1,// prettier-ignore
    READONLY = 0
}
export declare function downgradeRoleToNonSigner(role: AccountRole.READONLY_SIGNER): AccountRole.READONLY;
export declare function downgradeRoleToNonSigner(role: AccountRole.WRITABLE_SIGNER): AccountRole.WRITABLE;
export declare function downgradeRoleToNonSigner(role: AccountRole): AccountRole;
export declare function downgradeRoleToReadonly(role: AccountRole.WRITABLE): AccountRole.READONLY;
export declare function downgradeRoleToReadonly(role: AccountRole.WRITABLE_SIGNER): AccountRole.READONLY_SIGNER;
export declare function downgradeRoleToReadonly(role: AccountRole): AccountRole;
export declare function isSignerRole(role: AccountRole): role is AccountRole.READONLY_SIGNER | AccountRole.WRITABLE_SIGNER;
export declare function isWritableRole(role: AccountRole): role is AccountRole.WRITABLE | AccountRole.WRITABLE_SIGNER;
export declare function mergeRoles(roleA: AccountRole.WRITABLE, roleB: AccountRole.READONLY_SIGNER): AccountRole.WRITABLE_SIGNER;
export declare function mergeRoles(roleA: AccountRole.READONLY_SIGNER, roleB: AccountRole.WRITABLE): AccountRole.WRITABLE_SIGNER;
export declare function mergeRoles(roleA: AccountRole, roleB: AccountRole.WRITABLE_SIGNER): AccountRole.WRITABLE_SIGNER;
export declare function mergeRoles(roleA: AccountRole.WRITABLE_SIGNER, roleB: AccountRole): AccountRole.WRITABLE_SIGNER;
export declare function mergeRoles(roleA: AccountRole, roleB: AccountRole.READONLY_SIGNER): AccountRole.READONLY_SIGNER;
export declare function mergeRoles(roleA: AccountRole.READONLY_SIGNER, roleB: AccountRole): AccountRole.READONLY_SIGNER;
export declare function mergeRoles(roleA: AccountRole, roleB: AccountRole.WRITABLE): AccountRole.WRITABLE;
export declare function mergeRoles(roleA: AccountRole.WRITABLE, roleB: AccountRole): AccountRole.WRITABLE;
export declare function mergeRoles(roleA: AccountRole.READONLY, roleB: AccountRole.READONLY): AccountRole.READONLY;
export declare function mergeRoles(roleA: AccountRole, roleB: AccountRole): AccountRole;
export declare function upgradeRoleToSigner(role: AccountRole.READONLY): AccountRole.READONLY_SIGNER;
export declare function upgradeRoleToSigner(role: AccountRole.WRITABLE): AccountRole.WRITABLE_SIGNER;
export declare function upgradeRoleToSigner(role: AccountRole): AccountRole;
export declare function upgradeRoleToWritable(role: AccountRole.READONLY): AccountRole.WRITABLE;
export declare function upgradeRoleToWritable(role: AccountRole.READONLY_SIGNER): AccountRole.WRITABLE_SIGNER;
export declare function upgradeRoleToWritable(role: AccountRole): AccountRole;
//# sourceMappingURL=roles.d.ts.map