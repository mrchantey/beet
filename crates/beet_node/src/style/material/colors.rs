#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use beet_core::prelude::*;

// ── Primary ───────────────────────────────────────────────────────────────────

token2!(Primary, Color);
token2!(OnPrimary, Color);
token2!(PrimaryContainer, Color);
token2!(OnPrimaryContainer, Color);
token2!(InversePrimary, Color);
token2!(PrimaryFixed, Color);
token2!(PrimaryFixedDim, Color);
token2!(OnPrimaryFixed, Color);
token2!(OnPrimaryFixedVariant, Color);

// ── Secondary ─────────────────────────────────────────────────────────────────

token2!(Secondary, Color);
token2!(OnSecondary, Color);
token2!(SecondaryContainer, Color);
token2!(OnSecondaryContainer, Color);
token2!(SecondaryFixed, Color);
token2!(SecondaryFixedDim, Color);
token2!(OnSecondaryFixed, Color);
token2!(OnSecondaryFixedVariant, Color);

// ── Tertiary ──────────────────────────────────────────────────────────────────

token2!(Tertiary, Color);
token2!(OnTertiary, Color);
token2!(TertiaryContainer, Color);
token2!(OnTertiaryContainer, Color);
token2!(TertiaryFixed, Color);
token2!(TertiaryFixedDim, Color);
token2!(OnTertiaryFixed, Color);
token2!(OnTertiaryFixedVariant, Color);

// ── Error ─────────────────────────────────────────────────────────────────────

token2!(Error, Color);
token2!(OnError, Color);
token2!(ErrorContainer, Color);
token2!(OnErrorContainer, Color);

// ── Surface ───────────────────────────────────────────────────────────────────

token2!(SurfaceDim, Color);
token2!(Surface, Color);
token2!(SurfaceTint, Color);
token2!(SurfaceBright, Color);
token2!(SurfaceContainerLowest, Color);
token2!(SurfaceContainerLow, Color);
token2!(SurfaceContainer, Color);
token2!(SurfaceContainerHigh, Color);
token2!(SurfaceContainerHighest, Color);
token2!(OnSurface, Color);
token2!(OnSurfaceVariant, Color);
token2!(SurfaceVariant, Color);

// ── Outline ───────────────────────────────────────────────────────────────────

token2!(Outline, Color);
token2!(OutlineVariant, Color);

// ── Inverse ───────────────────────────────────────────────────────────────────

token2!(InverseSurface, Color);
token2!(InverseOnSurface, Color);

// ── Background ────────────────────────────────────────────────────────────────

token2!(Background, Color);
token2!(OnBackground, Color);

// ── Misc ──────────────────────────────────────────────────────────────────────

token2!(Shadow, Color);
token2!(Scrim, Color);

// ── Opacity scalars ───────────────────────────────────────────────────────────

token2!(OpacityHovered, f32);
token2!(OpacityFocused, f32);
token2!(OpacityPressed, f32);
token2!(OpacityDragged, f32);
