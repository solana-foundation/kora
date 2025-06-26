import type { Slot } from '@solana/rpc-types';
type RootNotificationsApiNotification = Slot;
export type RootNotificationsApi = {
    /**
     * Subscribe to receive notification anytime a new root is set by the validator
     */
    rootNotifications(NO_CONFIG?: Record<string, never>): RootNotificationsApiNotification;
};
export {};
//# sourceMappingURL=root-notifications.d.ts.map