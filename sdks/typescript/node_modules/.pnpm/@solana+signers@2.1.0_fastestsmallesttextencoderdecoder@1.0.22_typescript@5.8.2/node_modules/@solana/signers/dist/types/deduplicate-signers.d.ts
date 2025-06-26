import { MessageSigner } from './message-signer';
import { TransactionSigner } from './transaction-signer';
/**
 * Removes all duplicated {@link MessageSigner | MessageSigners} and
 * {@link TransactionSigner | TransactionSigners} from a provided array
 * by comparing their {@link Address | addresses}.
 *
 * @internal
 */
export declare function deduplicateSigners<TSigner extends MessageSigner | TransactionSigner>(signers: readonly TSigner[]): readonly TSigner[];
//# sourceMappingURL=deduplicate-signers.d.ts.map