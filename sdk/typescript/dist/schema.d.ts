import { z } from "zod";
export declare const Launchpad: z.ZodEnum<["clanker", "flaunch", "bankr", "zora"]>;
export type Launchpad = z.infer<typeof Launchpad>;
export declare const Deploy: z.ZodObject<{
    launchpad: z.ZodEnum<["clanker", "flaunch", "bankr", "zora"]>;
    token: z.ZodString;
    deployer: z.ZodString;
    block_number: z.ZodNumber;
    block_timestamp: z.ZodNumber;
    tx_hash: z.ZodString;
    log_index: z.ZodNumber;
    initial_supply: z.ZodOptional<z.ZodNullable<z.ZodString>>;
    raw: z.ZodOptional<z.ZodUnknown>;
}, "strip", z.ZodTypeAny, {
    launchpad: "clanker" | "flaunch" | "bankr" | "zora";
    token: string;
    deployer: string;
    block_number: number;
    block_timestamp: number;
    tx_hash: string;
    log_index: number;
    initial_supply?: string | null | undefined;
    raw?: unknown;
}, {
    launchpad: "clanker" | "flaunch" | "bankr" | "zora";
    token: string;
    deployer: string;
    block_number: number;
    block_timestamp: number;
    tx_hash: string;
    log_index: number;
    initial_supply?: string | null | undefined;
    raw?: unknown;
}>;
export type Deploy = z.infer<typeof Deploy>;
export declare const TOPIC_PREFIX = "shade.launches.";
export declare const allTopics: (lps: Launchpad[]) => string[];
//# sourceMappingURL=schema.d.ts.map