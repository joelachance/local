import type { MemkitConfig } from "./types.js";
export declare function getConfig(): MemkitConfig;
export declare function setConfig(config: Partial<MemkitConfig>): void;
export declare function clientGet(path: string): Promise<unknown>;
export declare function clientPost(path: string, body: unknown): Promise<unknown>;
//# sourceMappingURL=client.d.ts.map