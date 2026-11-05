# W-Connect design review — round 2

Reviewed 6 November 2026 against the rebuilt application at `http://127.0.0.1:7676`, with the new Settings page and airflow mark. **Overall: 9.33/10.** This remains a qualitative assessment of the supported local fan-control workflow, not a usability-study statistic. The numerical target did not override observed defects.

## Evidence and method

Used the same ten equally weighted categories as round 1. Inspected the actual browser UI at desktop1440px and1280px and narrow390px and360px layouts, including cooling, lighting, settings, light/dark appearance and reduced motion. Used local Playwright after the shared preview browser had already failed twice; did not operate another browser session or save settings to hardware.

Browser request interception provided explicit review fixtures for import/export, failed saves, pending/applied confirmation and unreachable-service states. All writes in those tests were fulfilled locally by the test harness. Actual hardware telemetry was used in the initial rendered-layout and keyboard tests. Screenshots, scripts and Axe results are retained in ignored `.research/review/`.

Ran Axe with WCAG2 A/AA and WCAG2.1 AA tags on all three views in both themes. Diagnostic script injection used a test-browser CSP bypass; application security settings were unchanged. Axe is a useful automated check, not a substitute for complete assistive-technology testing.

The competitor comparison remains the primary documentation/screenshots in [round1](design-review-1.md): [L-Connect3](https://lian-li.com/l-connect3/), [Fan Control](https://getfancontrol.com/), [CoolerControl](https://docs.coolercontrol.org/), and [OpenRGB](https://openrgb.org/). Their installed apps were not operated. W-Connect now addresses the key gaps exposed by those references—profile portability, appearance, meaningful settings and health feedback—while retaining a simpler, calmer workflow for its narrower hardware scope. Broader feature parity is not claimed.

## Fixed scorecard

| Category | Score | Observed result |
| --- | ---: | --- |
| Hierarchy and aesthetic craft | 9.4 | Operational headings, readable controls, consistent spacing and a cohesive frost/slate palette. The interface now looks like a small hardware instrument rather than a landing page. |
| Identity and logo | 9.5 | The airflow W, restrained status dot, favicon and branded startup animation form a coherent identity at their actual sizes. Navigation uses consistent line icons. |
| Cooling workflow | 9.5 | Bank selection and fan nodes survive polling; curve keyboard editing and undo work; curve settings remain staged. Zero-RPM banks are quieter and accurately labelled as lacking a speed signal. |
| Lighting workflow | 9.1 | Clear preview, effect selection, stepped brightness, editable hex value and validated staged changes. Keyboard bank selection still destroys the focused button, and the light-theme active bank label fails contrast. |
| Settings and completeness | 9.6 | Appearance and motion persist separately from cooling; interval changes stage normally; profile export/import and diagnostics work. Valid profile import causes no hardware write, and undo restores the prior state. Safeguards and background behavior are explained. |
| Feedback, errors and loading | 9.0 | Branded loading, retry guidance, draft retention on save failure and revision-based pending→applied confirmation work. Initial Full speed is visibly enabled but does nothing before state arrives; applied confirmation remains visible indefinitely in this tested build. |
| Motion | 9.6 | Fan SVG identity is stable across polls, so animation phase is not repeatedly reset. Reduced-motion preference disables animation. Startup and view transitions are restrained and related to the product. |
| Accessibility and keyboard | 8.8 | Cooling-bank focus survives4.3 seconds and chart focus survives2.5 seconds across polls; ArrowUp and undo work. Lighting-bank focus still drops to BODY after keyboard activation. Axe flags two active light-theme labels at4.12:1 instead of4.5:1. |
| Responsive layout | 9.3 | No horizontal overflow at360,390 or1280px; Settings stacks well, lighting controls remain usable, and chart axis text adapts rather than shrinking proportionally. On narrow cooling cards, the “100% sent” line wraps awkwardly. |
| Clarity and trust | 9.5 | Real telemetry, safe-temperature overrides, configured versus measured information, queued/applied distinction, controller-wide sync effects and crash limitations are communicated honestly. No invented values or unsupported capabilities were used to improve presentation. |
| **Mean** | **9.33** | Sum93.3 ÷10. |

## Concrete remaining corrections

1. **Keep lighting-bank buttons stable.** Reproduction: open Lighting, focus the third `#lighting-tabs button`, press Enter. `document.activeElement` becomes BODY because `renderChannels` rebuilds all lighting-tab HTML when selection changes. Update selected classes/attributes and labels in place; keep focus on the activated button through selection and subsequent polls.
2. **Correct active-label contrast in the light theme.** Axe reports the Balanced preset and active lighting-bank label at4.12:1 (`#367c96` text on `#e8f2f6`). Darken that text color or adjust the background to at least4.5:1, then rerun all six view/theme combinations.
3. **Disable the initial cooling override until state exists.** During startup the Full speed control appears available, but its event handler returns without a request because state is null. Disabled presentation should match its availability.
4. **Let applied confirmation settle.** A save correctly progresses from “Profile saved · applying to controller…” to “✓ Profile applied to controller”, but the completion overlay remains indefinitely. Auto-dismiss the successful acknowledgement after a readable interval; keep pending/failure information until resolved.
5. **Keep narrow-card output labels together.** At390px the “100% sent” label wraps internally. Put it on a deliberate second metric row or keep it together while retaining the RPM emphasis.

The parent was informed during the review and has prepared fixes. Those changes were not counted as already verified in this score. A further review should inspect the rebuilt version and repeat the focused checks, including interaction and contrast tests. No new design direction is needed; the remaining work is specific polish and accessibility closure.

## Tests that passed

- Cooling bank focus and original fan SVG node preserved across multiple polls.
- Curve point focus preserved across polling; ArrowUp changes the point and Undo restores it.
- Light/dark settings render correctly; reduced motion yields `animation-name: none`.
- Exported JSON contains the supported format/version and four-bank configuration.
- Valid import stages edits, performs zero hardware requests, and can be undone.
- Failed save reports its error and retains the edited bank name.
- Mocked save changes pending→applied only after the confirmed revision catches up.
- Hex entry updates the color preview; invalid hex sets `aria-invalid` and blocks saving.
- No document overflow at390,360 or1280px; no JavaScript page errors during the main inspection.
- Unreachable service reveals Retry connection with actionable background-service guidance.
- Axe found no A/AA violations on dark-theme cooling, lighting or settings, or on light-theme settings. The two light-theme contrast failures above are the only reported violations in this run.
