# W-Connect design review — round 3

Reviewed 6 November 2026 against the rebuilt application at `http://127.0.0.1:7676`. **Overall: 9.53/10.** The fixed review criterion of 9.5+ is met for the supported fan-control workflow. This is a qualitative design assessment, not a measured usability-study result or a claim that the product is universally superior to mature alternatives.

## Method and evidence

Used the same ten equally weighted categories established in [round 1](design-review-1.md), and retested every remaining finding from [round 2](design-review-2.md). Inspected the rendered app in local Chromium at 1440px, 1280px, 390px and 360px, using the already-authorized Playwright fallback after the shared browser failures. Checked cooling, lighting, settings, both palettes, normal/reduced motion, initial loading and an unreachable background service.

Tests began from a read of the running app's configuration. Browser request interception then supplied review fixtures for save failures, revision acknowledgements and profile operations. Every mutation request in those workflows was intercepted; **no fan or lighting setting was written to actual hardware**. Some screenshots intentionally contain fixture temperatures, so they are design evidence rather than hardware telemetry evidence.

Evidence is retained in ignored `.research/review/results/round3-*`; the browser scripts are `.research/review/round3.cjs` and `round3-axe.cjs`. All browsers opened by this reviewer were closed after inspection. No application code was changed by this reviewer.

## Fixed scorecard

| Category | Score | Observed result |
| --- | ---: | --- |
| Hierarchy and aesthetic craft | 9.4 | Cohesive frost/slate palettes, clear operating headings and readable controls. The layout remains intentionally spacious; on a phone, reaching the editor requires scrolling past the overview and banks. This is a modest density tradeoff rather than a broken workflow. |
| Identity and logo | 9.5 | The airflow W is recognizable in the sidebar and startup assembly, with matching favicon artwork. Line icons and the restrained status accent support a consistent identity without adding visual noise. |
| Cooling workflow | 9.5 | Stable bank selection, presets, draggable/keyboard/numeric editing and Undo form a direct workflow. Numeric editing now preserves focus when tabbing to the adjacent duty field. Drafts remain distinct from applied settings. |
| Lighting workflow | 9.5 | Lighting-bank activation and subsequent polling retain keyboard focus. Clear effect previews, color entry, stepped brightness and controller-wide sync explanation make the narrow supported workflow understandable. |
| Settings and completeness | 9.6 | Appearance, motion, control interval, profile import/export, diagnostics and safeguards have coherent homes. Local display preferences are separated from staged controller settings. Profile import performed zero hardware writes and could be undone. |
| Feedback, errors and loading | 9.6 | Branded startup explains what is happening; the initial override is correctly unavailable. Unreachable service reveals Retry. Failed saves retain drafts, successful saves progress from pending to confirmed application, and completion feedback then dismisses. |
| Motion | 9.6 | Fan SVG nodes remain stable across polling, preserving continuity. Loading and page transitions are restrained. Explicit reduced motion disables the animation rather than merely slowing it. |
| Accessibility and keyboard | 9.6 | Cooling banks, lighting banks, curve points and numeric controls retain usable focus. Axe reported zero WCAG A/AA violations across all six view/theme combinations. This does not substitute for a full assistive-technology audit. |
| Responsive layout | 9.5 | No horizontal overflow in any of the nine 1280/390/360px × cooling/lighting/settings checks. Mobile duty labels occupy a deliberate row and remain unbroken; chart labels and exact-value controls remain usable. |
| Clarity and trust | 9.5 | The UI distinguishes readings, requested output, staged settings and hardware application. No-RPM ambiguity, missing-sensor behavior, safety overrides, lighting sync consequences and background/crash behavior remain explicit. |
| **Mean** | **9.53** | Sum 95.3 ÷ 10. |

## Closure of round 2 findings

1. **Lighting-bank focus: passed.** Focus the third lighting-bank button and activate it with Enter. The same button remains focused immediately and after another 2.6 seconds of polling.
2. **Selected-label contrast: passed.** The deployed light-theme blue is now `#2e7089`. Axe found no A/AA violations in cooling, lighting or settings, in either light or dark appearance. The previous Balanced preset and lighting-bank contrast findings are gone.
3. **Initial override availability: passed.** With the initial state request held by the browser fixture, Full speed is disabled. It becomes available after valid state arrives.
4. **Applied notification expiry: passed.** A mocked save shows pending until `applied_revision` catches up, then shows confirmation and becomes hidden within the six-second display interval. A failed save retains the edited name and dirty state.
5. **Narrow output label: passed.** At 360px the `100% sent` label is on a separate metric row with `white-space: nowrap`; measured height was 17px. There is no internal word wrap or document overflow.

Additional checks passed: cooling focus and original fan node survived polling; a focused curve point survived a poll and ArrowUp changed its duty; numeric Tab moved to the adjacent input; profile export produced the expected format/version/four-bank payload; import staged changes without a POST; Undo returned to a clean profile; reduced motion computed `animation-name: none`; unreachable service provided actionable Retry guidance. No JavaScript page errors were recorded.

## Comparison and provisional ranking

The competitor evidence remains the official documentation and published screenshots collected in round 1: [L-Connect 3](https://lian-li.com/l-connect3/), [Fan Control](https://getfancontrol.com/), [CoolerControl](https://docs.coolercontrol.org/), and [OpenRGB](https://openrgb.org/). Their installed apps were **not** operated by this reviewer. The ordering below is therefore a provisional design preference for this user's simple daily fan-bank tuning task, not an equally tested product ranking.

| Preference for this task | Interface and interaction assessment | Where the alternative remains stronger |
| --- | --- | --- |
| **1 — W-Connect, for its supported hub** | The smallest conceptual load: select a bank, shape a response or lighting scene, review edits, save, and see confirmed application. The tested theme, motion and recovery details now match the visual polish. | This review establishes no broader hardware compatibility, Windows hardware validation, firmware features or long-term reliability. |
| **2 — CoolerControl / Fan Control, depending on platform and tuning depth** | Their documented interfaces offer stronger expert-oriented configuration and profile workflows. They expose more concepts, which is valuable for complex systems but less direct for the two populated banks in this setup. | Broader sensor/device and advanced cooling workflows; maturity and tuning depth should outweigh W-Connect's simpler presentation when those capabilities are needed. |
| **3 — L-Connect 3, for this narrow daily workflow** | Official screenshots show dense manufacturer-oriented navigation and many device-specific functions. W-Connect's focused bank/curve/settings layout is easier to scan for the task tested here. | Manufacturer-specific coverage and integration. The comparison does not claim W-Connect replaces every L-Connect function. |
| **Separate category — OpenRGB** | A conventional, explicit device/zone/color utility. W-Connect's bank preview and shared cooling/lighting flow are more approachable for this particular controller. | Broad cross-platform lighting scope. Ranking OpenRGB's fan-curve interaction would be unfair because that is not the role represented by its documented interface. |

The numerical score applies only to W-Connect's inspected interface. No numerical score is assigned to an unoperated competitor, and the hardware expansion work should preserve the tested clarity about supported versus unverified capabilities.

## Remaining limits

No blocking design or interaction defect was found in this round. The score does not certify every browser, assistive technology, locale, Windows install, controller model or failure mode. Further hardware support must retain clear model-specific capability and verification labels; it must not silently imply that this visual review tested those devices.
