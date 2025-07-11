import { parse } from "@std/toml";

export interface BeetConfig extends Record<string, unknown> {
  app: {
    name: string;
    domain: string;
    prod_stage: string;
  };
}



export async function readBeetConfig(configPath: string): Promise<BeetConfig> {
  const content = await Deno.readTextFile(configPath);
  const parsed = parse(content) as BeetConfig;
  return parsed;
}

// Tests
import { assertEquals } from "@std/assert";

Deno.test("readBeetConfig - can read beet.toml from workspace root", async () => {
  const configPath = "../../beet.toml";
  const config: BeetConfig = await readBeetConfig(configPath);

  assertEquals(config.app.name, "beet-site");
  assertEquals(config.app.domain, "beetstack.dev");
  assertEquals(config.app.prod_stage, "prod");
});
