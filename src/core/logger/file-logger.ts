import { createWriteStream, type WriteStream } from "node:fs";

import type { LoggerLike } from "../contracts";

export class FileLogger implements LoggerLike {
  private stream: WriteStream;

  constructor(public readonly filePath: string) {
    this.stream = createWriteStream(filePath, { flags: "a", encoding: "utf8" });
  }

  log(message: string): void {
    this.stream.write(`[${new Date().toISOString()}] ${message}\n`);
  }

  async close(): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      this.stream.once("error", reject);
      this.stream.end(() => resolve());
    });
  }
}
