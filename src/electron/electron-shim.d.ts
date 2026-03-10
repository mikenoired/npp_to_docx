declare module "electron" {
  export const app: {
    isPackaged: boolean;
    getPath(name: string): string;
    whenReady(): Promise<void>;
    on(event: string, listener: (...args: unknown[]) => void | Promise<void>): void;
    quit(): void;
  };

  export class BrowserWindow {
    constructor(options: {
      width?: number;
      height?: number;
      minWidth?: number;
      minHeight?: number;
      title?: string;
      webPreferences?: {
        contextIsolation?: boolean;
        nodeIntegration?: boolean;
        preload?: string;
      };
    });
    loadURL(url: string): Promise<void>;
    loadFile(path: string): Promise<void>;
    webContents: {
      openDevTools(options?: { mode?: string }): void;
    };
    static getAllWindows(): BrowserWindow[];
  }

  export const ipcMain: {
    handle(channel: string, listener: (event: unknown, payload?: unknown) => unknown): void;
  };

  export const ipcRenderer: {
    invoke(channel: string, payload?: unknown): Promise<unknown>;
  };

  export const contextBridge: {
    exposeInMainWorld(key: string, value: unknown): void;
  };

  export const dialog: {
    showOpenDialog(options: {
      title?: string;
      defaultPath?: string;
      properties: string[];
    }): Promise<{ canceled: boolean; filePaths: string[] }>;
  };
}
