#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
		.insert(Primary)
		.insert(OnPrimary)
		.insert(PrimaryContainer)
		.insert(OnPrimaryContainer)
		.insert(InversePrimary)
		.insert(PrimaryFixed)
		.insert(PrimaryFixedDim)
		.insert(OnPrimaryFixed)
		.insert(OnPrimaryFixedVariant)

		.insert(Secondary)
		.insert(OnSecondary)
		.insert(SecondaryContainer)
		.insert(OnSecondaryContainer)
		.insert(SecondaryFixed)
		.insert(SecondaryFixedDim)
		.insert(OnSecondaryFixed)
		.insert(OnSecondaryFixedVariant)

		.insert(Tertiary)
		.insert(OnTertiary)
		.insert(TertiaryContainer)
		.insert(OnTertiaryContainer)
		.insert(TertiaryFixed)
		.insert(TertiaryFixedDim)
		.insert(OnTertiaryFixed)
		.insert(OnTertiaryFixedVariant)

		.insert(Error)
		.insert(OnError)
		.insert(ErrorContainer)
		.insert(OnErrorContainer)

 	  .insert(SurfaceDim)
    .insert(Surface)
    .insert(SurfaceTint)
    .insert(SurfaceBright)
    .insert(SurfaceContainerLowest)
    .insert(SurfaceContainerLow)
    .insert(SurfaceContainer)
    .insert(SurfaceContainerHigh)
    .insert(SurfaceContainerHighest)
    .insert(OnSurface)
    .insert(OnSurfaceVariant)
    .insert(SurfaceVariant)

    .insert(Outline)
    .insert(OutlineVariant)

    .insert(InverseSurface)
    .insert(InverseOnSurface)

    .insert(Background)
    .insert(OnBackground)

    .insert(Shadow)
    .insert(Scrim)

    .insert(OpacityHovered)
    .insert(OpacityFocused)
    .insert(OpacityPressed)
    .insert(OpacityDragged)

}

// ── Primary ───────────────────────────────────────────────────────────────────
css_variable!(PrimaryRole, ColorRole);

css_variable!(Primary, Color);
css_variable!(OnPrimary, Color);

css_variable!(PrimaryContainer, Color);
css_variable!(OnPrimaryContainer, Color);
css_variable!(InversePrimary, Color);
css_variable!(PrimaryFixed, Color);
css_variable!(PrimaryFixedDim, Color);
css_variable!(OnPrimaryFixed, Color);
css_variable!(OnPrimaryFixedVariant, Color);

// ── Secondary ─────────────────────────────────────────────────────────────────

css_variable!(Secondary, Color);
css_variable!(OnSecondary, Color);
css_variable!(SecondaryContainer, Color);
css_variable!(OnSecondaryContainer, Color);
css_variable!(SecondaryFixed, Color);
css_variable!(SecondaryFixedDim, Color);
css_variable!(OnSecondaryFixed, Color);
css_variable!(OnSecondaryFixedVariant, Color);

// ── Tertiary ──────────────────────────────────────────────────────────────────

css_variable!(Tertiary, Color);
css_variable!(OnTertiary, Color);
css_variable!(TertiaryContainer, Color);
css_variable!(OnTertiaryContainer, Color);
css_variable!(TertiaryFixed, Color);
css_variable!(TertiaryFixedDim, Color);
css_variable!(OnTertiaryFixed, Color);
css_variable!(OnTertiaryFixedVariant, Color);

// ── Error ─────────────────────────────────────────────────────────────────────

css_variable!(Error, Color);
css_variable!(OnError, Color);
css_variable!(ErrorContainer, Color);
css_variable!(OnErrorContainer, Color);

// ── Surface ───────────────────────────────────────────────────────────────────

css_variable!(SurfaceDim, Color);
css_variable!(Surface, Color);
css_variable!(SurfaceTint, Color);
css_variable!(SurfaceBright, Color);
css_variable!(SurfaceContainerLowest, Color);
css_variable!(SurfaceContainerLow, Color);
css_variable!(SurfaceContainer, Color);
css_variable!(SurfaceContainerHigh, Color);
css_variable!(SurfaceContainerHighest, Color);
css_variable!(OnSurface, Color);
css_variable!(OnSurfaceVariant, Color);
css_variable!(SurfaceVariant, Color);

// ── Outline ───────────────────────────────────────────────────────────────────

css_variable!(Outline, Color);
css_variable!(OutlineVariant, Color);

// ── Inverse ───────────────────────────────────────────────────────────────────

css_variable!(InverseSurface, Color);
css_variable!(InverseOnSurface, Color);

// ── Background ────────────────────────────────────────────────────────────────

css_variable!(Background, Color);
css_variable!(OnBackground, Color);

// ── Misc ──────────────────────────────────────────────────────────────────────

css_variable!(Shadow, Color);
css_variable!(Scrim, Color);

// ── Opacity scalars ───────────────────────────────────────────────────────────

css_variable!(OpacityHovered, f32);
css_variable!(OpacityFocused, f32);
css_variable!(OpacityPressed, f32);
css_variable!(OpacityDragged, f32);
