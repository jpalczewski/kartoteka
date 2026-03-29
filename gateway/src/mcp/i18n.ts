import { FluentBundle, FluentResource } from "@fluent/bundle";

// FTL files are copied as .txt during prebuild so esbuild's built-in text loader handles them.
import enMcp from "../../locales/en/mcp.txt";
import plMcp from "../../locales/pl/mcp.txt";

function createBundle(locale: string, ftlText: string): FluentBundle {
  const bundle = new FluentBundle(locale);
  const resource = new FluentResource(ftlText);
  bundle.addResource(resource);
  return bundle;
}

const bundles: Record<string, FluentBundle> = {
  en: createBundle("en", enMcp),
  pl: createBundle("pl", plMcp),
};

/**
 * Translate a key to the given locale.
 * Falls back to "en" if the locale bundle doesn't have the key.
 * Falls back to the key itself if not found in any bundle.
 */
export function tr(key: string, locale: string): string {
  const bundle = bundles[locale] ?? bundles["en"];
  const msg = bundle.getMessage(key);
  if (msg?.value) {
    return bundle.formatPattern(msg.value);
  }
  // Fallback to English
  const enMsg = bundles["en"].getMessage(key);
  if (enMsg?.value) {
    return bundles["en"].formatPattern(enMsg.value);
  }
  return key;
}
