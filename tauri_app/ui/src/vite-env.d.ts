/// <reference types="vite/client" />

declare module "@novnc/novnc" {
  export default class RFB {
    scaleViewport: boolean;
    resizeSession: boolean;
    constructor(target: HTMLElement, url: string, options?: Record<string, unknown>);
    addEventListener(type: string, listener: (event: CustomEvent) => void): void;
    removeEventListener(type: string, listener: (event: CustomEvent) => void): void;
    disconnect(): void;
  }
}
