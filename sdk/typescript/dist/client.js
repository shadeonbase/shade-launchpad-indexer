import { Kafka, logLevel } from "kafkajs";
import { Deploy, allTopics } from "./schema.js";
export class IndexerClient {
    kafka;
    opts;
    consumer;
    closed = false;
    constructor(opts) {
        if (!opts.brokers.length)
            throw new Error("brokers must be non-empty");
        this.opts = { ...opts };
        this.kafka = new Kafka({
            clientId: `shade-indexer-${opts.groupId}`,
            brokers: opts.brokers,
            logLevel: logLevel.WARN,
        });
    }
    /** Async-iterator over decoded deploy events. Stops when [[close]] is called. */
    async *stream() {
        const consumer = this.kafka.consumer({
            groupId: this.opts.groupId,
            sessionTimeout: 30_000,
            heartbeatInterval: 3_000,
        });
        this.consumer = consumer;
        await consumer.connect();
        const topics = allTopics(this.opts.launchpads ?? ["clanker", "flaunch", "bankr", "zora"]);
        for (const topic of topics) {
            await consumer.subscribe({ topic, fromBeginning: !!this.opts.fromBeginning });
        }
        const queue = [];
        const waiters = [];
        const push = (d) => {
            const w = waiters.shift();
            if (w)
                w(d);
            else
                queue.push(d);
        };
        await consumer.run({
            eachMessage: async ({ message }) => {
                if (!message.value)
                    return;
                const raw = message.value.toString();
                let parsedJson;
                try {
                    parsedJson = JSON.parse(raw);
                }
                catch (e) {
                    this.opts.onParseError?.(raw, e);
                    return;
                }
                const result = Deploy.safeParse(parsedJson);
                if (!result.success) {
                    this.opts.onParseError?.(raw, result.error);
                    return;
                }
                push(result.data);
            },
        });
        while (!this.closed) {
            if (queue.length) {
                const next = queue.shift();
                if (next)
                    yield next;
                continue;
            }
            const next = await new Promise((resolve) => waiters.push(resolve));
            yield next;
        }
    }
    async close() {
        this.closed = true;
        await this.consumer?.disconnect();
    }
}
//# sourceMappingURL=client.js.map