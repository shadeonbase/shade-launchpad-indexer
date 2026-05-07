import { z } from "zod";
export const Launchpad = z.enum(["clanker", "flaunch", "bankr", "zora"]);
const Address = z.string().regex(/^0x[a-fA-F0-9]{40}$/, "0x-prefixed 20-byte address");
const TxHash = z.string().regex(/^0x[a-fA-F0-9]{64}$/, "0x-prefixed 32-byte hash");
export const Deploy = z.object({
    launchpad: Launchpad,
    token: Address,
    deployer: Address,
    block_number: z.number().int().nonnegative(),
    block_timestamp: z.number().int().nonnegative(),
    tx_hash: TxHash,
    log_index: z.number().int().nonnegative(),
    initial_supply: z.string().nullable().optional(),
    raw: z.unknown().optional(),
});
export const TOPIC_PREFIX = "shade.launches.";
export const allTopics = (lps) => lps.map((l) => `${TOPIC_PREFIX}${l}`);
//# sourceMappingURL=schema.js.map