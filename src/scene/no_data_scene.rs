use crate::common::Color;
use crate::framework::context::Context;
use crate::framework::error::{GameError, GameResult};
use crate::framework::graphics;
use crate::game::shared_game_state::SharedGameState;
use crate::graphics::font::Font;
use crate::scene::Scene;

pub struct NoDataScene {
    #[cfg(target_os = "android")]
    flag: bool,
    err: String,
}

impl NoDataScene {
    pub fn new(err: GameError) -> Self {
        Self {
            #[cfg(target_os = "android")]
            flag: false,
            err: err.to_string(),
        }
    }
}

impl Scene for NoDataScene {
    #[allow(unused)]
    fn tick(&mut self, state: &mut SharedGameState, ctx: &mut Context) -> GameResult {
        #[cfg(target_os = "android")]
        {
            if !self.flag {
                self.flag = true;
                if let Err(error) = crate::framework::android_locale::show_data_error(&self.err) {
                    log::warn!("Cannot show native data error: {}", error);
                }
            }

        }
        Ok(())
    }

    fn draw(&self, state: &mut SharedGameState, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, Color::from_rgb(30, 0, 0));
        // Android supplies localized guidance using system fonts, including when
        // the game's own font data is unavailable.
        if cfg!(target_os = "android") { return Ok(()); }

        state.font.builder().center(state.canvas_size.0).y(10.0).color((255, 100, 100, 255)).draw(
            "CaveStory-rs internal error",
            ctx,
            &state.constants,
            &mut state.texture_set,
        )?;

        state.font.builder().center(state.canvas_size.0).y(30.0).color((255, 100, 100, 255)).draw(
            "Failed to load game data.",
            ctx,
            &state.constants,
            &mut state.texture_set,
        )?;

        let mut y = 60.0;

        {
            // put max 80 chars per line
            let mut lines = Vec::new();
            let mut line = String::new();

            for word in self.err.split(' ') {
                let combined_word = line.clone() + " " + word;
                let line_length = state.font.compute_width(&mut combined_word.chars(), None);

                if line_length > state.canvas_size.0 as f32 {
                    lines.push(line);
                    line = String::new();
                }

                line.push_str(word);
                line.push(' ');
            }
            lines.push(line);

            for line in lines {
                state.font.builder().center(state.canvas_size.0).y(y).draw(
                    &line,
                    ctx,
                    &state.constants,
                    &mut state.texture_set,
                )?;
                y += 20.0;
            }
        }

        Ok(())
    }
}
