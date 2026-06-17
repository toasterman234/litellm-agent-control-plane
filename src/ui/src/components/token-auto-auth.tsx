"use client";

import { useEffect } from "react";
import { setStoredMasterKey, getStoredMasterKey } from "@/lib/api";

// Listens for { type: "litellm-auth", encrypted_token: "..." } postMessage
// from the litellm parent frame.  Forwards the ciphertext to the LAP's own
// /api/plugin-auth endpoint for server-side decryption using LITELLM_SALT_KEY.
// The raw litellm credential never appears in browser JS — only the ciphertext
// crosses the iframe boundary.
export default function TokenAutoAuth() {
  useEffect(() => {
    const handler = async (event: MessageEvent) => {
      if (event.data?.type !== "litellm-auth") return;

      const encrypted = event.data.encrypted_token as string | undefined;
      if (!encrypted) return;

      try {
        const res = await fetch("/api/plugin-auth", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ encrypted_token: encrypted }),
        });
        if (!res.ok) return;
        const data = await res.json();
        const token: string | undefined = data?.token;
        if (token && token !== getStoredMasterKey()) {
          setStoredMasterKey(token);
          // Reload so the app re-initialises with the new credential.
          window.location.reload();
        }
      } catch {
        // Silently ignore — user can sign in manually.
      }
    };

    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, []);

  return null;
}
