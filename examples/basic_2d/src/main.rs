use seishin2d::prelude::*;

struct Basic2D {
    player_texture: Texture,
    player: Sprite,
    camera: Camera2D,
    beep: Option<AssetHandle<SoundAsset>>,
}

impl Game2D for Basic2D {
    fn new(ctx: &mut StartupContext) -> GameResult<Self> {
        let player_texture = ctx.load_texture("sprites/player.png")?;
        let player = Sprite::new(
            player_texture.id(),
            Transform2D::from_translation(0.0, 0.0),
            96.0,
            96.0,
        );

        if let Some(error) = ctx.audio_backend_error() {
            println!("audio unavailable, demo will continue silently: {error}");
        }

        let beep = match ctx.load_sound("audio/beep.wav") {
            Ok(sound) => Some(sound),
            Err(error) => {
                println!("audio asset unavailable, demo will continue silently: {error}");
                None
            }
        };

        println!("use arrow keys to move, Space for sound, Escape to quit");

        Ok(Self {
            player_texture,
            player,
            camera: Camera2D::default(),
            beep,
        })
    }

    fn update(&mut self, ctx: &mut FrameContext<'_>) -> GameResult<()> {
        let speed = 180.0 * ctx.delta_seconds();
        self.player.transform.x += ctx.axis(KeyCode::ArrowLeft, KeyCode::ArrowRight) * speed;
        self.player.transform.y += ctx.axis(KeyCode::ArrowUp, KeyCode::ArrowDown) * speed;

        if ctx.input().just_pressed(KeyCode::Space) {
            if let Some(sound) = self.beep {
                println!("audio: {:?}", ctx.play_sound(sound));
            }
        }

        if ctx.frame() % 60 == 0 {
            println!(
                "frame {} player ({:.1}, {:.1})",
                ctx.frame(),
                self.player.transform.x,
                self.player.transform.y
            );
        }

        Ok(())
    }

    fn render(&self, ctx: &mut RenderContext) {
        ctx.clear(ClearColor::CORNFLOWER);
        ctx.camera(self.camera);
        ctx.texture(&self.player_texture);
        ctx.sprite(self.player);
    }

    fn shutdown(&mut self) -> GameResult<()> {
        println!("shutdown cleanly");
        Ok(())
    }
}

fn main() -> GameResult<()> {
    App::new("seishin2d basic 2d")
        .window_size(960, 540)
        .asset_root(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
        .run::<Basic2D>()
}
