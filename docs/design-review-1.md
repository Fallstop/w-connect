# W-Connect design review — round 1

Reviewed 6 November 2026 against the installed application at `http://127.0.0.1:7676`, before the second design pass. **Overall: 7.21/10.** This is an independent qualitative design assessment, not a usability-study statistic or a negotiated target.

## Method and evidence

The same ten equally weighted categories below should be used in later rounds. A 9.5 means exceptional polish for the supported UNI FAN SL workflow, with no material remaining interaction, feedback, accessibility or responsive-layout defects. It does not require matching unrelated controllers, wireless fans, LCDs or firmware features.

Inspected the live app at 1440×1000 and 390×844, including cooling, lighting and system pages; read its HTML/CSS/JavaScript; exercised keyboard focus without saving any hardware changes. Browser snapshots failed twice (`Preview automation snapshot timed out after 15000ms`, then `Preview automation client server-browser disconnected during snapshot`), so review continued in local Playwright Chromium. Screenshots and scripts are retained in ignored `.research/review/results/` and `.research/review/`.

Competitor comparisons use official pages and published interface screenshots opened in a browser. Their installed applications were **not** operated during this review; judgments about workflow breadth come from documentation, and judgments about appearance come from screenshots.

## Fixed scorecard

| Category | Score | Evidence and limiting factor |
| --- | ---: | --- |
| Hierarchy and aesthetic craft | 8.0 | Cohesive slate/ice palette, restrained cards and readable grouping. Too much space is allocated to a promotional heading while the actual controls use 9–11px text. |
| Identity and logo | 6.5 | A clean but generic zigzag W and miscellaneous Unicode icons; little relationship to airflow, connected fan banks or the instrument aesthetic. |
| Cooling workflow | 7.8 | Direct bank selection, three presets, draggable/numeric curves and explicit thermal protection. Empty ports get equal visual weight and three illustrated fans despite zero measured RPM. |
| Lighting workflow | 8.0 | Clear bank selection, effect/color/brightness controls, staged application and sync consequence explained. Preview is attractive but the rainbow effect is three static unrelated colors; fan-count configuration is on a different page. |
| Settings and completeness | 4.5 | System page is diagnostics only. No appearance or motion preference, profile backup/import, or editable sampling preference. |
| Feedback, errors and loading | 8.0 | Honest unavailable readings, staged Save/Discard and thermal warning. Generic spinner and limited recovery guidance; queued changes versus hardware application mainly require consulting activity. |
| Motion | 6.5 | Fan motion is tied to measured RPM and honors system reduced motion. Entire fan DOM is replaced every poll, restarting animations. Loading has no product identity. |
| Accessibility and keyboard | 6.8 | Labels, focus styling, numeric curve editing and keyboard point movement exist. A focused bank button loses focus to BODY after the next poll; many labels/help texts are tiny and pale. |
| Responsive layout | 7.8 | No horizontal overflow at 390px; layout stacks coherently. Main curve editing is far down the page, and scaled SVG axis labels become extremely small. |
| Clarity and trust | 8.2 | CPU/GPU/RPM provenance and critical-temperature override are explicit. Unused ports look like configured working banks, the header is promotional instead of operational, and fan-count help says display-only although the count also configures lighting. |
| **Mean** | **7.21** | Sum72.1 ÷10. |

## Competitor comparison

| Product and primary evidence | Strength visible in the evidence | Implication for W-Connect |
| --- | --- | --- |
| [L-Connect3 official page](https://lian-li.com/l-connect3/) and [published settings screen](https://lian-li.com/wp-content/uploads/2025/11/WXWorkCapture_17624211536954.png) | Manufacturer-specific fan/RGB monitoring and device coverage; official instructions explicitly provide export/import of lighting and fan settings. Published screens are dense and use a strongly branded dark interface. | W-Connect already feels calmer for this two-bank setup, but settings backup/restore should exist before claiming a more complete daily workflow. No claim of broader feature parity is justified. |
| [Fan Control](https://getfancontrol.com/) and its [published interface](https://getfancontrol.com/assets/main_light.png) | Clearly exposes controls and assigned curves, themes, multiple configurations, assisted setup and detailed tuning. Its dense cards favor expert adjustment. | Retain direct access to curve controls and give presets/settings visible homes. W-Connect can be easier for one known hub without trying to reproduce every advanced parameter. |
| [CoolerControl documentation](https://docs.coolercontrol.org/) and [5.0 interface](https://docs.coolercontrol.org/images/whats-new/5-0/home-full.webp) | Daemon/device health, configuration workflows, profile setup and accessible troubleshooting are first-class. Documentation confirms a daemon-served Web UI. | Present background control, sensor health and recovery with the same seriousness as the visual tuning tools. Avoid implying a lightweight browser architecture alone is novel. |
| [OpenRGB](https://openrgb.org/) and its [published interface](https://openrgb.org/OpenRGB-0.9.56cd18bb.webp) | Cross-platform lighting with explicit device/zone selection, color controls and profile operations; conventional utility styling. | W-Connect's bank preview is more approachable for its narrow scope. Match the clarity of scope and profile handling; do not confuse polish with OpenRGB's broader capabilities. |

These are comparisons of observable strengths, not numerical ratings of untested competing apps. The supplied L-Connect installer analysis in `vendor-reverse-engineering.md` is separate technical evidence and does not establish subjective UX superiority.

## Prioritized corrections

1. **Preserve interactive DOM across polling.** Confirmed reproduction: focus the first `.channel-card`, wait2.6 seconds, then `document.activeElement.tagName` is `BODY`. The same redraw strategy replaces curve points and resets fan animations. Update telemetry text/properties in place; rebuild only when structure changes, retaining focus when needed.
2. **Build a useful Settings page.** Add appearance (system/light/dark), motion preference, sampling interval, export/import of the saved configuration, and clear startup/safety facts. Imported profiles must be validated and staged for explicit Save, with no silent hardware application. Distinguish local display preferences from controller settings.
3. **Make operating controls readable.** Aim for14px body/control text and12px secondary text, with adequate contrast and touch areas. Small uppercase metadata may be smaller sparingly; temperature/RPM units and safeguards are not decorative. Mobile numeric editing must remain clear without depending on scaled-down SVG labels.
4. **Replace the generic identity and loading treatment.** Use a restrained airflow/connected-bank mark consistently in the sidebar, favicon and loading state. Loading should identify the actual task (connecting to the local service), use a lightweight purposeful animation, respect reduced motion and offer an actionable unavailable state.
5. **Make populated versus unpopulated channels honest.** De-emphasize channels without RPM, label the ambiguity accurately (unused or no RPM), and avoid implying three discovered fans. Fan counts are user configuration, not discovery. Fix help copy: fan count also configures lighting.
6. **Improve operational feedback.** Show a stable pending/saved/applied or failed distinction for changes, without claiming command acknowledgement before the control loop runs. Maintain full-speed explanation in both cooling and lighting. Keep draft changes during polling and emergency/resume.
7. **Refine the lighting preview and motion.** Make effect previews recognizably represent the selected mode while labeling them as previews. Preserve fan animation phase across telemetry updates. Provide reduced/off motion without hiding important state.
8. **Replace promotional headings with useful context.** The space above the banks can explain current cooling state or the chosen profile. Keep any personality in concise product details rather than repeated slogans and generic workspace labels.

## Acceptance for a later 9.5+ assessment

Re-review the rendered, rebuilt app rather than edited source alone. Evaluate desktop and narrow phone layouts, light/dark/system appearances, ordinary and reduced motion, initial load, unreachable service, missing sensors, dirty draft, save failure, and queued/applied feedback. Keyboard bank/point focus must survive multiple polls. Navigation, theme and import/export must work without hardware writes during inspection. Logo/favicon/loading should be coherent at their actual sizes. Exact curve values must remain usable on mobile; primary controls should not rely on tiny text. No invented telemetry, implied unsupported hardware or hidden unsafe setting may be introduced to improve appearance.

The score must follow those observations. Achieving the requested numerical target alone is not evidence that defects have been resolved.
