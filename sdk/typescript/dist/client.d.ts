import { Deploy, type Launchpad } from "./schema.js";
export interface IndexerClientOptions {
    brokers: string[];
    groupId: string;
    launchpads?: Launchpad[];
    /** Read from earliest offset on first start (default: false). */
    fromBeginning?: boolean;
    /** Called with non-fatal parse errors so consumers can ship to telemetry. */
    onParseError?: (raw: string, error: unknown) => void;
}
export declare class IndexerClient {
    private readonly kafka;
    private readonly opts;
    private consumer?;
    private closed;
    constructor(opts: IndexerClientOptions);
    /** Async-iterator over decoded deploy events. Stops when [[close]] is called. */
    stream(): AsyncGenerator<Deploy, void, void>;
    close(): Promise<void>;
}
//# sourceMappingURL=client.d.ts.map