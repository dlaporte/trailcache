#!/usr/bin/env python3
"""
Generate 40-character AI summaries for merit badge requirements.

Usage:
    export ANTHROPIC_API_KEY=your_key
    python scripts/generate_summaries.py

Reads: raw_requirements.json
Writes: data/requirement_summaries.json
"""

import json
import os
import sys
import time
from pathlib import Path

try:
    import anthropic
except ImportError:
    print("Please install the anthropic package: pip install anthropic")
    sys.exit(1)

# Configuration
INPUT_FILE = Path(__file__).parent.parent / "raw_requirements.json"
OUTPUT_FILE = Path(__file__).parent.parent / "data" / "requirement_summaries.json"
CHECKPOINT_FILE = Path(__file__).parent.parent / "data" / "summaries_checkpoint.json"
MAX_SUMMARY_LENGTH = 40
BATCH_SIZE = 50  # Save checkpoint every N summaries
MODEL = "claude-sonnet-4-20250514"

def load_requirements():
    """Load all requirements from the raw dump."""
    with open(INPUT_FILE) as f:
        data = json.load(f)

    # Extract all unique requirement texts
    unique_texts = {}
    for badge in data:
        badge_name = badge["name"]
        for version in badge.get("versions", []):
            for req in version.get("requirements", []):
                text = req.get("text", "").strip()
                if text and text not in unique_texts:
                    # Skip HTML notes
                    if text.startswith("<b>NOTE:</b>") or text.startswith("NOTE:"):
                        continue
                    unique_texts[text] = {
                        "badge": badge_name,
                        "number": req.get("number", ""),
                    }

    return unique_texts

def load_checkpoint():
    """Load existing summaries from checkpoint."""
    if CHECKPOINT_FILE.exists():
        with open(CHECKPOINT_FILE) as f:
            return json.load(f)
    return {}

def save_checkpoint(summaries):
    """Save summaries to checkpoint file."""
    CHECKPOINT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(CHECKPOINT_FILE, "w") as f:
        json.dump(summaries, f, indent=2)

def generate_summary(client, text, badge_name, req_number):
    """Generate a 40-char summary for a requirement."""

    # If already short enough, return as-is
    if len(text) <= MAX_SUMMARY_LENGTH:
        return {"summary": text, "flag": None}

    prompt = f"""Summarize this Boy Scout merit badge requirement in EXACTLY 40 characters or less.
The summary should capture the core action/knowledge required.
Use abbreviations if needed (e.g., "Demo" for "Demonstrate", "Explain" for "Explain to your counselor").
Do NOT include the requirement number.
Do NOT use quotes around the summary.

Badge: {badge_name}
Requirement {req_number}: {text}

If critical information MUST be lost to fit 40 chars, add a note.

Respond in this exact JSON format:
{{"summary": "your 40 char max summary", "flag": null}}

Or if critical info is lost:
{{"summary": "your 40 char max summary", "flag": "brief note about what's lost"}}"""

    try:
        response = client.messages.create(
            model=MODEL,
            max_tokens=150,
            messages=[{"role": "user", "content": prompt}]
        )

        result_text = response.content[0].text.strip()

        # Parse JSON response
        # Handle potential markdown code blocks
        if result_text.startswith("```"):
            result_text = result_text.split("```")[1]
            if result_text.startswith("json"):
                result_text = result_text[4:]

        result = json.loads(result_text)

        # Validate length
        summary = result.get("summary", "")
        if len(summary) > MAX_SUMMARY_LENGTH:
            # Truncate with ellipsis if API returned too long
            summary = summary[:MAX_SUMMARY_LENGTH-1] + "…"
            result["summary"] = summary
            if not result.get("flag"):
                result["flag"] = "auto-truncated"

        return result

    except Exception as e:
        print(f"  Error: {e}")
        # Fallback: truncate original
        return {
            "summary": text[:MAX_SUMMARY_LENGTH-1] + "…",
            "flag": f"API error: {str(e)[:50]}"
        }

def main():
    # Check for API key
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("Error: ANTHROPIC_API_KEY environment variable not set")
        sys.exit(1)

    client = anthropic.Anthropic()

    print("Loading requirements...")
    unique_texts = load_requirements()
    print(f"Found {len(unique_texts)} unique requirement texts")

    # Load checkpoint
    summaries = load_checkpoint()
    print(f"Loaded {len(summaries)} existing summaries from checkpoint")

    # Filter to only texts that need processing
    to_process = {k: v for k, v in unique_texts.items() if k not in summaries}
    print(f"Need to generate {len(to_process)} new summaries")

    if not to_process:
        print("All summaries already generated!")
    else:
        # Process remaining
        flagged = []
        processed = 0

        for text, info in to_process.items():
            processed += 1
            badge = info["badge"]
            number = info["number"]

            print(f"[{processed}/{len(to_process)}] {badge} {number}: ", end="", flush=True)

            result = generate_summary(client, text, badge, number)
            summaries[text] = result

            print(f"{result['summary'][:50]}...")

            if result.get("flag"):
                flagged.append({
                    "badge": badge,
                    "number": number,
                    "original": text[:100] + "..." if len(text) > 100 else text,
                    "summary": result["summary"],
                    "flag": result["flag"]
                })

            # Save checkpoint periodically
            if processed % BATCH_SIZE == 0:
                print(f"  Saving checkpoint ({processed} processed)...")
                save_checkpoint(summaries)

            # Rate limiting - be nice to the API
            time.sleep(0.1)

        # Final save
        save_checkpoint(summaries)

        # Report flagged items
        if flagged:
            print(f"\n{'='*60}")
            print(f"FLAGGED ITEMS ({len(flagged)} items may have lost critical meaning):")
            print('='*60)
            for item in flagged:
                print(f"\n{item['badge']} {item['number']}:")
                print(f"  Original: {item['original']}")
                print(f"  Summary:  {item['summary']}")
                print(f"  Flag:     {item['flag']}")

    # Build final output file
    print(f"\nBuilding final output file...")
    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)

    # Create a compact lookup structure
    output = {
        "version": "1.0",
        "generated": time.strftime("%Y-%m-%d"),
        "summaries": {k: v["summary"] for k, v in summaries.items()},
        "flags": {k: v["flag"] for k, v in summaries.items() if v.get("flag")}
    }

    with open(OUTPUT_FILE, "w") as f:
        json.dump(output, f, indent=2)

    print(f"Wrote {len(output['summaries'])} summaries to {OUTPUT_FILE}")
    print(f"Flagged items: {len(output['flags'])}")
    print("Done!")

if __name__ == "__main__":
    main()
