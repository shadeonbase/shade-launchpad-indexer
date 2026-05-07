import { Kafka, type Consumer, type EachMessagePayload, logLevel } from "kafkajs";
import { Deploy, allTopics, type Launchpad } from "./schema.js";

export interface IndexerClientOptions {
  brokers:     string[];
  groupId:     string;
  launchpads?: Launchpad[];
  /** Read from earliest offset on first start (default: false). */
  fromBeginning?: boolean;
  /** Called with non-fatal parse errors so consumers can ship to telemetry. */
  onParseError?: (raw: string, error: unknown) => void;
}

export class IndexerClient {
  private readonly kafka: Kafka;
  private readonly opts: Required<Pick<IndexerClientOptions, "brokers" | "groupId">> &
    IndexerClientOptions;
  private consumer?: Consumer;
  private closed = false;

  constructor(opts: IndexerClientOptions) {
    if (!opts.brokers.length) throw new Error("brokers must be non-empty");
    this.opts = { ...opts };
    this.kafka = new Kafka({
      clientId: `shade-indexer-${opts.groupId}`,
      brokers:  opts.brokers,
      logLevel: logLevel.WARN,
    });
  }

  /** Async-iterator over decoded deploy events. Stops when [[close]] is called. */
  async *stream(): AsyncGenerator<Deploy, void, void> {
    const consumer = this.kafka.consumer({
      groupId:           this.opts.groupId,
      sessionTimeout:    30_000,
      heartbeatInterval: 3_000,
    });
    this.consumer = consumer;
    await consumer.connect();

    const topics = allTopics(
      this.opts.launchpads ?? (["clanker", "flaunch", "bankr", "zora"] as Launchpad[]),
    );
    for (const topic of topics) {
      await consumer.subscribe({ topic, fromBeginning: !!this.opts.fromBeginning });
    }

    const queue: Deploy[] = [];
    const waiters: Array<(d: Deploy) => void> = [];

    const push = (d: Deploy) => {
      const w = waiters.shift();
      if (w) w(d);
      else queue.push(d);
    };

    await consumer.run({
      eachMessage: async ({ message }: EachMessagePayload) => {
        if (!message.value) return;
        const raw = message.value.toString();
        let parsedJson: unknown;
        try {
          parsedJson = JSON.parse(raw);
        } catch (e) {
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
        if (next) yield next;
        continue;
      }
      const next = await new Promise<Deploy>((resolve) => waiters.push(resolve));
      yield next;
    }
  }

  async close(): Promise<void> {
    this.closed = true;
    await this.consumer?.disconnect();
  }
}
