#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use beet_core::prelude::*;

// ── Primary ───────────────────────────────────────────────────────────────────

token!(Primary, Color);
token!(OnPrimary, Color);
token!(PrimaryContainer, Color);
token!(OnPrimaryContainer, Color);
token!(InversePrimary, Color);
token!(PrimaryFixed, Color);
token!(PrimaryFixedDim, Color);
token!(OnPrimaryFixed, Color);
token!(OnPrimaryFixedVariant, Color);

// ── Secondary ─────────────────────────────────────────────────────────────────

token!(Secondary, Color);
token!(OnSecondary, Color);
token!(SecondaryContainer, Color);
token!(OnSecondaryContainer, Color);
token!(SecondaryFixed, Color);
token!(SecondaryFixedDim, Color);
token!(OnSecondaryFixed, Color);
token!(OnSecondaryFixedVariant, Color);

// ── Tertiary ──────────────────────────────────────────────────────────────────

token!(Tertiary, Color);
token!(OnTertiary, Color);
token!(TertiaryContainer, Color);
token!(OnTertiaryContainer, Color);
token!(TertiaryFixed, Color);
token!(TertiaryFixedDim, Color);
token!(OnTertiaryFixed, Color);
token!(OnTertiaryFixedVariant, Color);

// ── Error ─────────────────────────────────────────────────────────────────────

token!(Error, Color);
token!(OnError, Color);
token!(ErrorContainer, Color);
token!(OnErrorContainer, Color);

// ── Surface ───────────────────────────────────────────────────────────────────

token!(SurfaceDim, Color);
token!(Surface, Color);
token!(SurfaceTint, Color);
token!(SurfaceBright, Color);
token!(SurfaceContainerLowest, Color);
token!(SurfaceContainerLow, Color);
token!(SurfaceContainer, Color);
token!(SurfaceContainerHigh, Color);
token!(SurfaceContainerHighest, Color);
token!(OnSurface, Color);
token!(OnSurfaceVariant, Color);
token!(SurfaceVariant, Color);

// ── Outline ───────────────────────────────────────────────────────────────────

token!(Outline, Color);
token!(OutlineVariant, Color);

// ── Inverse ───────────────────────────────────────────────────────────────────

token!(InverseSurface, Color);
token!(InverseOnSurface, Color);

// ── Background ────────────────────────────────────────────────────────────────

token!(Background, Color);
token!(OnBackground, Color);

// ── Misc ──────────────────────────────────────────────────────────────────────

token!(Shadow, Color);
token!(Scrim, Color);

// ── Opacity scalars ───────────────────────────────────────────────────────────

token!(OpacityHovered, f32);
token!(OpacityFocused, f32);
token!(OpacityPressed, f32);
token!(OpacityDragged, f32);
