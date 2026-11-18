/// <reference types="vite/client" />

declare module "@novnc/novnc" {
  export default class RFB {
    scaleViewport: boolean;
    resizeSession: boolean;
    clipViewport: boolean;
    viewOnly: boolean;
    constructor(target: HTMLElement, url: string, options?: Record<string, unknown>);
    addEventListener(type: string, listener: (event: CustomEvent) => void): void;
    removeEventListener(type: string, listener: (event: CustomEvent) => void): void;
    sendCtrlAltDel(): void;
    sendKey(keysym: number, code: string, down?: boolean): void;
    focus(options?: FocusOptions): void;
    disconnect(): void;
  }
}
