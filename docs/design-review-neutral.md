# Neutral-theme aesthetic review — 6 November 2026

The earlier 9.53/10 review mixed functionality, accessibility and appearance. It should not be read as an aesthetic score for the current interface. The user rejected that visual direction, especially the saturated dark surfaces, low text contrast, large warning banner and lighting illustration.

## Method

Every review below used a new agent with **no inherited conversation**. Each received only nine screenshots and the same rubric. The reviewer could not read source, previous reviews, repository history, change summaries or other agents' messages. Neither the desired score nor a previous score appeared in the review brief. All reviews are recorded, including decreases.

The screenshots covered cooling, lighting and settings in both themes, a 390 px lighting layout, initial loading, and the expanded status disclosure. Desktop captures were 1440 × 1000. Hardware telemetry was live; lighting edits remained unsaved. The screenshots were captured in a browser with reduced motion, so the aesthetic reviews do not establish animation quality.

| Category | Weight |
| --- | --- |
| Color / contrast | 25% |
| Typography | 20% |
| Composition / spacing | 20% |
| Controls / hardware illustration | 20% |
| Coherence across screens, sizes, brand and loading | 15% |

The rubric excluded feature count, engineering effort, accessibility results and presumed implementation quality. Ten meant exceptional professional visual craft.

## Results

| Blind round | Color | Type | Composition | Controls / art | Coherence | Weighted total |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 8.0 | 7.5 | 7.0 | 7.6 | 7.7 | 7.575 |
| 2 | 8.0 | 7.4 | 7.0 | 8.0 | 7.7 | 7.635 |
| 3 | 8.0 | 7.5 | 7.0 | 8.0 | 8.5 | 7.775 |
| 4 | 7.8 | 7.4 | 7.0 | 7.6 | 8.0 | 7.550 |
| 5 | 8.1 | 7.7 | 7.5 | 8.0 | 8.2 | 7.890 |

The requested 9.5 aesthetic threshold has **not** been established. These are independent subjective assessments, not a usability study or proof of an incremental improvement. Differences between reviewers are visible in the results.

The strongest recurring findings were the fan construction, restrained palette, readable cooling hierarchy, consistent themes and small W mark. Recurring weaknesses concerned spatial balance, secondary typography and ordinary form-control styling. Reviewers also noted that a status popover covers content while open; this remains a deliberate overlay rather than inserting a warning into the page and moving controls.

Iterations replaced the colored theme with neutral charcoal, removed redundant copy, increased control text, rebuilt the fan illustration, grouped the lighting editor, revised settings, and changed the narrow layout to a compact bank selector. Save has a separate layout row and cannot overlap the scrolling controls. Loading is centered and has reserved error/retry positions. Final small refinements distinguish connecting from connected, vary the fan blade positions and add restrained light spill.

## Separate behavioral validation

Browser checks exercised all three views in both themes, five widths from 360 to 1440 px, status changes, keyboard chart editing, draft retention, reduced motion and footer separation. They found no JavaScript errors, horizontal overflow or axe WCAG A/AA findings. Normal, hot, missing-sensor, disconnected and recovered status states kept the same header, main, channel-grid and chart geometry.

The application has 16 Rust tests and 14 DOM tests. The extracted Linux package smoke test includes both embedded font routes, discovery, origin protection, override and saving in USB-free demo mode inside an isolated network namespace. UI testing and screenshot capture do not send mutations to the real controller.
