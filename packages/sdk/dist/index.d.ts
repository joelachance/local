import type { QueryOptions, QueryResult } from "./types.js";
export declare function configure(options: {
    url?: string;
}): void;
export declare function memkit(model: string): {
    model: string;
    tools: unknown[];
};
export declare function query(text: string, options?: QueryOptions): Promise<QueryResult>;
export declare function executeTool(name: string, args: Record<string, unknown>): Promise<string>;
export declare function add(items: string | string[] | Array<{
    role: string;
    content: string;
}>): Promise<void>;
declare const defaultExport: typeof memkit & {
    configure: typeof configure;
    query: typeof query;
    add: typeof add;
    executeTool: typeof executeTool;
};
export default defaultExport;
//# sourceMappingURL=index.d.ts.map