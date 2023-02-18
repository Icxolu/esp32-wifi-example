use embedded_graphics::{
    prelude::*,
    primitives::{Arc, PrimitiveStyle},
};

pub struct Loader<C: PixelColor> {
    arc: Arc,
    style: PrimitiveStyle<C>,
    progress: f32,
}

impl<C: PixelColor> Loader<C> {
    pub fn new(center: Point, diameter: u32, style: PrimitiveStyle<C>) -> Self {
        Self {
            arc: Arc::with_center(center, diameter, 0.0.deg(), 360.0.deg()),
            style,
            progress: 0.0,
        }
    }

    pub fn update(&mut self, delta: core::time::Duration) {
        const SPEED: f32 = 2.0;

        self.progress = (self.progress + delta.as_secs_f32()) % SPEED;
        let factor = self.progress / SPEED;

        self.arc.angle_start = (720.0 / SPEED * self.progress).deg();
        self.arc.angle_sweep = (10.0 + 270.0 * (core::f32::consts::PI * factor).sin()).deg();
    }
}

impl<C: PixelColor> Drawable for Loader<C> {
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.arc.into_styled(self.style).draw(target)
    }
}
