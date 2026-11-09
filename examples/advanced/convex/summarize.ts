import { internalAction } from "./_generated/server";
import { internal } from "./_generated/api";
import { v } from "convex/values";

/**
 * Scheduled by `bookmarks:create`. Fetches the page, summarizes it, and patches the
 * bookmark to `ready` (or `failed`). Internal, so it isn't callable from Rust and
 * doesn't appear in the generated bindings.
 */
export const run = internalAction({
  args: { id: v.id("bookmarks"), url: v.string() },
  returns: v.null(),
  handler: async (ctx, { id, url }) => {
    try {
      const page = extractPage(await fetchPage(url));
      const result = (await summarizeWithLlm(page)) ?? summarizeWithoutLlm(page);
      await ctx.runMutation(internal.bookmarks.markReady, {
        id,
        title: page.title || new URL(url).hostname,
        ...result,
      });
    } catch (e) {
      const error = e instanceof Error ? e.message : String(e);
      console.error(`bookmark ${id}: ${error}`);
      await ctx.runMutation(internal.bookmarks.markFailed, { id, error });
    }
    return null;
  },
});

type Page = { title: string; description: string; text: string };
type Summary = { summary: string; tags: string[]; summarizer: "llm" | "fallback" };

const MAX_HTML_BYTES = 1_000_000;

async function fetchPage(url: string): Promise<string> {
  const res = await fetch(url, {
    headers: { "user-agent": "convex-typegen-bookmarks/1.0", accept: "text/html" },
    signal: AbortSignal.timeout(15_000),
  });
  if (!res.ok) {
    throw new Error(`fetching the page returned HTTP ${res.status}`);
  }
  return (await res.text()).slice(0, MAX_HTML_BYTES);
}

function extractPage(html: string): Page {
  const meta = (name: string) =>
    html.match(new RegExp(`<meta[^>]+(?:name|property)=["']${name}["'][^>]*>`, "i"))?.[0].match(/content=["']([^"']*)["']/i)?.[1];
  // `||`, not `??`: pages often ship empty meta tags.
  const title = meta("og:title") || html.match(/<title[^>]*>([\s\S]*?)<\/title>/i)?.[1] || "";
  const body = (html.match(/<body[^>]*>([\s\S]*)<\/body>/i)?.[1] ?? html)
    .replace(/<(script|style|noscript|svg)[^>]*>[\s\S]*?<\/\1>/gi, " ");
  const paragraphs = [...body.matchAll(/<p[^>]*>([\s\S]*?)<\/p>/gi)].map((m) => clean(m[1]));
  const description = meta("description") || meta("og:description") || paragraphs.find((p) => p.length > 40) || "";
  return { title: clean(title), description: clean(description), text: clean(body).slice(0, 8_000) };
}

function clean(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;|&apos;/g, "'")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Any OpenAI-compatible chat completions endpoint (OpenAI, Ollama, LM Studio, ...).
 * Returns `null` when `LLM_BASE_URL` and `LLM_MODEL` aren't set, so the fallback runs.
 */
async function summarizeWithLlm(page: Page): Promise<Summary | null> {
  const baseUrl = process.env.LLM_BASE_URL;
  const model = process.env.LLM_MODEL;
  if (!baseUrl || !model) {
    return null;
  }
  const apiKey = process.env.LLM_API_KEY;
  const res = await fetch(`${baseUrl.replace(/\/+$/, "")}/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json", ...(apiKey ? { authorization: `Bearer ${apiKey}` } : {}) },
    signal: AbortSignal.timeout(60_000),
    body: JSON.stringify({
      model,
      temperature: 0,
      response_format: { type: "json_object" },
      messages: [
        {
          role: "system",
          // A concrete example keeps small local models on the expected shape.
          content:
            'Summarize a web page for a bookmark. Reply with a JSON object with exactly two keys: "summary" (one or two ' +
            'sentences) and "tags" (an array of 3 to 5 short lowercase topic words). Example: ' +
            '{"summary": "A guide to baking sourdough bread at home.", "tags": ["baking", "bread", "recipes"]}',
        },
        { role: "user", content: `Title: ${page.title}\nDescription: ${page.description}\n\n${page.text.slice(0, 4_000)}` },
      ],
    }),
  });
  if (!res.ok) {
    throw new Error(`the LLM endpoint returned HTTP ${res.status}`);
  }
  const body = await res.json();
  const parsed = JSON.parse(body.choices?.[0]?.message?.content ?? "{}");
  if (typeof parsed.summary !== "string" || !Array.isArray(parsed.tags)) {
    throw new Error("the LLM reply did not contain a summary and tags");
  }
  const tags: string[] = parsed.tags.filter((t: unknown): t is string => typeof t === "string");
  return { summary: parsed.summary.trim(), tags: normalizeTags(tags), summarizer: "llm" };
}

const STOPWORDS = new Set(
  ("a about after all also an and any are as at be been but by can could do for from has have how if in into is it its " +
    "just may more most must no not of on one only or other our out over said should so some such than that the their them " +
    "then there these they this those through to under up use used using very was we were what when which while who will " +
    "with would you your")
    .split(" "),
);

/** Deterministic and offline: the page's description plus its most frequent keywords. */
function summarizeWithoutLlm(page: Page): Summary {
  const source = page.description || page.text || page.title;
  const summary = source.length > 280 ? `${source.slice(0, 277).trimEnd()}...` : source;
  const counts = new Map<string, number>();
  // Title words count three times: they say most about the page.
  const words = `${page.title} ${page.title} ${page.title} ${page.description} ${page.text}`.toLowerCase().match(/[a-z][a-z0-9-]{2,}/g) ?? [];
  for (const word of words) {
    if (!STOPWORDS.has(word)) {
      counts.set(word, (counts.get(word) ?? 0) + 1);
    }
  }
  const tags = [...counts.entries()]
    .sort(([a, x], [b, y]) => y - x || a.localeCompare(b))
    .slice(0, 4)
    .map(([word]) => word);
  return { summary: summary || "(no description)", tags: normalizeTags(tags), summarizer: "fallback" };
}

function normalizeTags(tags: string[]): string[] {
  const out = tags.map((t) => t.trim().toLowerCase().replace(/\s+/g, "-")).filter((t) => t.length > 0 && t.length <= 32);
  return [...new Set(out)].slice(0, 5);
}
