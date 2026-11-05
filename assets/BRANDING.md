# Brand assets

The shared identity is an original emerald/ivory pixel microphone with a dark
stand, stepped sound marks and a small voxel foot. It is not the official Plasmo
Voice or Minecraft logo and contains no copied game textures.

- `icon-source.png`: selected original illustration; kept unchanged.
- `icon-32.png`, `icon-40.png`, `icon-64.png`: small UI exports.
- `icon-128.png`: README logo at its displayed resolution.
- `icon-256.png`: source for the prepared Horizon catalog icon (under 256 KiB).
- `icon-512.png`: larger logo export.
- `github-social.png`: 1280 × 640 GitHub social-preview artwork.
- `github-social.svg`: banner layout with the shared icon and original pixel wordmark.

Regenerate exports with `python3 tools/export_branding.py`. This uses point
sampling for sizes and librsvg for the layout. Illustration generation and its
refinement used the built-in **imagegen** tool; no API-key/CLI fallback was used.

## Generation prompt

Use case: logo-brand. Asset type: original square icon for an unofficial Plasmo Voice-compatible Horizon mod, shown at 40 x 40 pixels in a Minecraft launcher and larger in a GitHub README. Primary request: a carefully centered Minecraft inventory-item-style pixel-art microphone emblem. Make one coherent silhouette: an ivory and emerald microphone capsule in a chunky dark charcoal U-shaped stand, with two small stepped sound-wave marks. The base of the stand has a subtle tiny voxel / block shape, integrating the Minecraft feel into the microphone itself. Style: genuinely chunky 32 x 32-style pixel art, deliberate square pixels and hard stepped corners, restrained 3-shade material shading as an original Minecraft-inspired inventory sprite. Use the existing mod microphone colors: charcoal outline #20282b, pale warm ivory highlights, emerald green #79e38d. Strong contrast and a very readable silhouette even as a small app icon. Composition: square, front-facing, symmetric, compact, safe transparent margin around every side, the emblem occupies about 80 percent of the canvas. Transparent background with real alpha; no backdrop, text, letters, labels, watermark, gradients, bloom, smooth vector curves, 3D render, blur, or copied Minecraft textures. This must be original artwork, not the official Plasmo Voice or Minecraft logo.

## Refinement prompt

Use case: precise-object-edit. Edit this original microphone mod logo. Preserve exactly the centered microphone, its U-shaped dark stand, the tiny emerald/ivory voxel foot, and green sound marks. Keep the palette and transparent background. Make the artwork genuinely crisp Minecraft inventory PIXEL ART: every edge and every shading boundary must follow the SAME coarse square pixel grid, roughly a 32 x 32 sprite enlarged with nearest-neighbor. Replace all smooth gradients and grain with three flat color steps per material. Turn the diagonal faces of the tiny cube into stepped pixel diagonals. Simplify tiny unnecessary interior detail so the microphone remains readable at 40px. Keep the silhouette strongly outlined and balanced. No new objects, no lettering, no glow, no soft shadow, no anti-aliased or painted edges. Real transparent alpha outside the icon.
