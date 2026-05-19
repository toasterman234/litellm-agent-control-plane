/**
 * Composer image-paste helpers.
 *
 * The session composer accepts images pasted from the clipboard and forwards
 * them to the harness as Claude-format multimodal parts on the next
 * `POST /sessions/{id}/message_stream` — mirroring how Slack image uploads
 * are already shipped via `CreateSessionBody.initial_attachments`.
 *
 * Limits intentionally match the server-side `INITIAL_ATTACHMENT_*` constants
 * in `src/server/types.ts` so a paste that would be rejected by the API
 * surfaces a friendly inline error before the user hits send.
 */
import type { HarnessMessagePart } from "@/lib/api";

export const MAX_IMAGES_PER_MESSAGE = 10;
export const MAX_IMAGE_BYTES = 5 * 1024 * 1024;
export const ALLOWED_IMAGE_MIME = [
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
] as const;

export type AllowedImageMime = (typeof ALLOWED_IMAGE_MIME)[number];

export interface PastedImage {
  /** Stable per-paste id used as React key + remove handle. */
  id: string;
  mime_type: AllowedImageMime;
  /** Raw base64 payload (no `data:` prefix). */
  base64: string;
  /** Decoded byte length, for the preview chip label. */
  size_bytes: number;
  /** Original filename if the clipboard item carried one, else null. */
  name: string | null;
}

export interface ReadPastedImagesResult {
  images: PastedImage[];
  /** Per-item rejection reasons, ready to render under the composer. */
  errors: string[];
}

function isAllowedMime(m: string): m is AllowedImageMime {
  return (ALLOWED_IMAGE_MIME as readonly string[]).includes(m);
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error("FileReader returned non-string result"));
        return;
      }
      // result is `data:<mime>;base64,<payload>` — strip the prefix.
      const comma = result.indexOf(",");
      resolve(comma === -1 ? result : result.slice(comma + 1));
    };
    reader.onerror = () => reject(reader.error ?? new Error("read failed"));
    reader.readAsDataURL(blob);
  });
}

/**
 * Scan a clipboard `DataTransferItemList` for image entries, validate each
 * against the per-image cap and the MIME whitelist, and return the accepted
 * images + a list of human-readable rejections. Non-image items are silently
 * ignored so plain-text paste continues to fall through to the textarea.
 *
 * `existingCount` is used to enforce the overall per-message ceiling alongside
 * what's already in the composer's pending list.
 */
export async function readPastedImages(
  items: DataTransferItemList,
  existingCount: number,
): Promise<ReadPastedImagesResult> {
  const out: PastedImage[] = [];
  const errors: string[] = [];

  // DataTransferItemList isn't iterable in older lib targets; pull entries
  // out into a plain array first so we can `for...of` it.
  const collected: DataTransferItem[] = [];
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (item.kind === "file") collected.push(item);
  }

  let remaining = Math.max(0, MAX_IMAGES_PER_MESSAGE - existingCount);

  for (const item of collected) {
    if (remaining <= 0) {
      errors.push(`max ${MAX_IMAGES_PER_MESSAGE} images per message`);
      break;
    }
    const file = item.getAsFile();
    if (!file) continue;
    const mime = file.type;
    if (!isAllowedMime(mime)) {
      errors.push(
        `${file.name || "image"}: unsupported type ${mime || "unknown"}`,
      );
      continue;
    }
    if (file.size > MAX_IMAGE_BYTES) {
      errors.push(
        `${file.name || "image"}: ${formatBytes(file.size)} exceeds ${formatBytes(MAX_IMAGE_BYTES)}`,
      );
      continue;
    }
    let base64: string;
    try {
      base64 = await blobToBase64(file);
    } catch (e) {
      errors.push(
        `${file.name || "image"}: read failed (${e instanceof Error ? e.message : "unknown"})`,
      );
      continue;
    }
    out.push({
      id: `img-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      mime_type: mime,
      base64,
      size_bytes: file.size,
      name: file.name || null,
    });
    remaining--;
  }

  return { images: out, errors };
}

/**
 * Compose the harness-format parts array for a send that mixes a text prompt
 * with pasted images. Matches the shape that `runInitialPrompt` already
 * builds for `initial_attachments` on session create
 * (`src/app/api/v1/managed_agents/agents/[agent_id]/session/route.ts`), so the
 * harness side has zero new wire format to learn.
 *
 * Text is emitted first when non-empty so the conventional Claude
 * "instruction then attachments" ordering holds; an empty text is dropped
 * rather than sent as a blank part.
 */
export function buildMultimodalParts(
  text: string,
  images: readonly PastedImage[],
): HarnessMessagePart[] {
  const trimmed = text.trim();
  const parts: HarnessMessagePart[] = [];
  if (trimmed.length > 0) {
    parts.push({ type: "text", text: trimmed });
  }
  for (const img of images) {
    parts.push({
      type: "image",
      source: {
        type: "base64",
        media_type: img.mime_type,
        data: img.base64,
      },
    });
  }
  return parts;
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Build the `data:` URL the preview chip and message-bubble thumbnail use
 * to render an image without re-encoding. Pure function — kept here so any
 * future image part rendered server-side can use the same helper.
 */
export function imagePartToDataUrl(part: {
  source?: { type?: unknown; media_type?: unknown; data?: unknown };
}): string | null {
  const src = part.source;
  if (!src || typeof src !== "object") return null;
  if (src.type !== "base64") return null;
  const mime = typeof src.media_type === "string" ? src.media_type : null;
  const data = typeof src.data === "string" ? src.data : null;
  if (!mime || !data) return null;
  return `data:${mime};base64,${data}`;
}
