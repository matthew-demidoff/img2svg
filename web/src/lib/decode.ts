export interface DecodedImage {
  rgba: Uint8Array;
  width: number;
  height: number;
}

const MAX_EDGE = 2000;

function fittedSize(width: number, height: number): { width: number; height: number } {
  const longest = Math.max(width, height);
  if (longest <= MAX_EDGE) {
    return { width, height };
  }
  const scale = MAX_EDGE / longest;
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

export async function decode(file: File): Promise<DecodedImage> {
  const probe = await createImageBitmap(file);
  const { width, height } = fittedSize(probe.width, probe.height);

  // Re-decode at the target size so the browser does the downscale; the probe
  // bitmap is no longer needed once we know the source dimensions.
  const bitmap =
    width === probe.width && height === probe.height
      ? probe
      : await createImageBitmap(file, {
          resizeWidth: width,
          resizeHeight: height,
          resizeQuality: "high",
        });
  if (bitmap !== probe) {
    probe.close();
  }

  const canvas = new OffscreenCanvas(width, height);
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    bitmap.close();
    throw new Error("2D canvas context unavailable for image decode");
  }
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close();

  const { data } = ctx.getImageData(0, 0, width, height);
  return { rgba: new Uint8Array(data.buffer.slice(0)), width, height };
}
