import { IndexerClient } from "../src/index.js";

const client = new IndexerClient({
  brokers:    (process.env.KAFKA_BROKERS ?? "localhost:9092").split(","),
  groupId:    "shade-example",
  launchpads: ["clanker", "flaunch"],
  onParseError: (raw, err) => console.error("[parse-error]", err, raw.slice(0, 200)),
});

const stop = () => client.close().then(() => process.exit(0));
process.on("SIGINT",  stop);
process.on("SIGTERM", stop);

for await (const deploy of client.stream()) {
  console.log(
    `[${deploy.launchpad}] token=${deploy.token} deployer=${deploy.deployer} ` +
      `block=${deploy.block_number} ts=${deploy.block_timestamp}`,
  );
}
