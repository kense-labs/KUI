interface PreviewWindow extends Window {
  kuiPreview?: {
    controlRadius(): number | undefined;
    accent?(light: number, dark: number): void;
    iconIntent(dark: boolean): void;
    copyPicture(destination: HTMLCanvasElement): boolean;
    show(story: string, lang: string): void;
    preferences(dark: boolean, reduced: boolean, waitForPaint?: boolean): Promise<void>;
  };
}

export function previewRuntime(frame: HTMLIFrameElement) {
  return (frame.contentWindow as PreviewWindow | null)?.kuiPreview;
}
