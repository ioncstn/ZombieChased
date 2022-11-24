use ggez::{
    event,
    GameResult,
    graphics::{self, Color},
    Context,
    glam::*,
    input::keyboard::KeyInput,
    input::keyboard::KeyCode,
};

use cgmath::Point2;

use collision::{
    Aabb2,
    dbvt::DynamicBoundingVolumeTree,
    dbvt::TreeValue,
    dbvt::DiscreteVisitor,
};

use rand::{thread_rng, Rng};

use libm::{atan2f, sqrt};

use settings::{WIN_WIDTH, WIN_HEIGHT, PI, PX_MOVEMENT, BULLET_SPEED, PISTOL_RELOAD_TIME, PLAYER_HEIGHT, BULLET_HEIGHT, ENEMY_SPEED, ENEMY_COOLDOWN, PLAYER_WIDTH, ENEMY_WIDTH, BULLET_TIME, BULLET_WIDTH, BULLETS_SHOT, BULLETS_ANGLE, FOG_DISTANCE, ENEMY_FRAME_TIME, PARTICLE_HEALTH, PARTICLE_ANGLE, PLAYER_FRAME_TIME, MG_RELOAD_TIME};
mod settings;

fn vec_from_angle(angle: f32) -> Vec2 {
    let vx = angle.sin();
    let vy = angle.cos();
    Vec2::new(vx, vy)
}

fn distance(e1: &Entity, e2: &Entity) -> f32{
    let dist = e1.pos - e2.pos;
    sqrt((dist.x * dist.x + dist.y * dist.y) as f64) as f32
}

#[derive(Debug, Clone)]
enum EntityTypes{
    Player,
    Bullet,
    Enemy,
    Particle,
}

#[derive(PartialEq, Eq, Hash)]
enum Guns{
    Pistol,
    MachineGun,
}

#[derive(PartialEq)]
enum State{
    Playing,
    Paused,
    Unpausing,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Entity{
    entity_type: EntityTypes,
    pos: Vec2,
    d: Vec2,
    image: graphics::Image,
    health: u16,
    rotation: f32,
    frame: u8,
    frame_time: u8,
}

struct MainState {
    key_pressed: std::collections::HashMap<ggez::input::keyboard::KeyCode, f32>,
    mouse_pos: Vec2,
    player: Entity,
    particles: Vec<Entity>,
    bullets: Vec<Entity>,
    enemies: Vec<Entity>,
    cursor: graphics::Image,
    counter: u16,
    reloading: u16,
    bg: graphics::Image,
    paused_bg: graphics::Image,
    state: State,
    dollars: u16,
    guns: std::collections::HashMap<Guns, u8>,
    using_gun: Guns,
    //egui: EguiBackend,
}

impl MainState{


    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let player = Entity {
            entity_type: EntityTypes::Player,
            pos: Vec2::new(WIN_WIDTH / 2f32, WIN_HEIGHT / 2f32),
            rotation: 0f32,
            image: graphics::Image::from_path(ctx, "/pl1.png")?,
            d: Vec2::ZERO,
            health: 100,
            frame: 0,
            frame_time: PLAYER_FRAME_TIME,
        };
        let bg = graphics::Image::from_path(ctx, "/backg.png")?;
        let cursor = graphics::Image::from_path(ctx, "/cursor.png")?;

        ggez::input::mouse::set_cursor_hidden(ctx, true);

        //let egui = EguiBackend::new(ctx);

        let mut guns = std::collections::HashMap::new();
        guns.insert(Guns::Pistol, 2);
        guns.insert(Guns::MachineGun, 0);
        let using_gun = Guns::Pistol;

        let mut key_pressed = std::collections::HashMap::new();
        key_pressed.insert(KeyCode::W, 0f32);
        key_pressed.insert(KeyCode::D, 0f32);
        key_pressed.insert(KeyCode::A, 0f32);
        key_pressed.insert(KeyCode::S, 0f32);
        key_pressed.insert(KeyCode::Space, 0f32);

        let mouse_pos = Vec2::new(WIN_WIDTH / 2f32, WIN_WIDTH);

        let bullets = Vec::<Entity>::new();
        let enemies = Vec::<Entity>::new();
        let particles = Vec::<Entity>::new();
        let state = State::Playing;
        let dollars = 199;
        let paused_bg = graphics::Image::from_path(ctx, "/paused_bg.png").unwrap();

        Ok(MainState { using_gun, guns, paused_bg, dollars, state, player, reloading: 0, key_pressed, mouse_pos, cursor, bullets, counter: 60, enemies, bg, particles })
    }

    fn fire_shot(&mut self, ctx: &mut Context) -> GameResult{

        for _ in 0..BULLETS_SHOT{
            let x = self.player.pos.x;
            let y = self.player.pos.y;
            //random in 20 degrees cone:
            let randf = rand::random::<f32>() * BULLETS_ANGLE - BULLETS_ANGLE / 2f32;
            let rot = self.player.rotation + randf;
            let dir = vec_from_angle(-rot);
            let new_bullet = Entity{
                entity_type: EntityTypes::Bullet,
                pos: Vec2::new(x + dir.x * (BULLET_HEIGHT + PLAYER_HEIGHT) / 2f32, y + dir.y * (BULLET_HEIGHT + PLAYER_HEIGHT) / 2f32),
                d: Vec2::new(dir.x * BULLET_SPEED, dir.y * BULLET_SPEED),
                rotation: rot,
                health: BULLET_TIME,
                image: graphics::Image::from_path(ctx, "/bullet.png")?,
                frame: 0,
                frame_time: 0,
            };
            let rt: u16;
            match self.using_gun{
                Guns::Pistol => {rt = PISTOL_RELOAD_TIME; }
                Guns::MachineGun => {rt = MG_RELOAD_TIME; }
            }
            self.reloading = rt;
            self.bullets.push(new_bullet);
        }
        Ok(())
    }

    fn spawn_enemy(&mut self, ctx: &mut Context) -> GameResult{

        let x = (thread_rng().gen_range(0..=1) as f32) * WIN_WIDTH;
        let y = thread_rng().gen_range(0f32..=WIN_HEIGHT);
        let rot = atan2f(self.player.pos.y - y, self.player.pos.x - x) - PI / 2f32;
        let dir = vec_from_angle(-rot);
        let new_enemy = Entity{
            entity_type: EntityTypes::Enemy,
            pos: Vec2 {x, y},
            d: Vec2 { x: dir.x * ENEMY_SPEED, y: dir.y * ENEMY_SPEED },
            rotation: rot,
            image: graphics::Image::from_path(ctx, "/enemy.png")?,
            health: 1,
            frame: 0,
            frame_time: ENEMY_FRAME_TIME,
        };
        self.counter = ENEMY_COOLDOWN;
        self.enemies.push(new_enemy);
        
        Ok(())
    }

    fn clear_entities(&mut self) {

        self.bullets.retain(
            |bullet|
            bullet.health > 0
        );
        self.enemies.retain(
            |enemy|
            enemy.health == 1
        );
        self.particles.retain(
            |enemy|
            enemy.health > 0
        );
    }

    fn handle_bounderies(&mut self){

        let x = self.player.pos.x;
        let y = self.player.pos.y;
        //360
        if x < 0f32 {
            *self.key_pressed.get_mut(&KeyCode::A).unwrap() = 0f32;
        }
        //220
        if y < 0f32 {
            *self.key_pressed.get_mut(&KeyCode::W).unwrap() = 0f32;
        }
        //920
        if x > 1280f32 {
            *self.key_pressed.get_mut(&KeyCode::D).unwrap() = 0f32;
        }
        //500
        if y > 720f32 {
            *self.key_pressed.get_mut(&KeyCode::S).unwrap() = 0f32;
        }
    }

    fn handle_collisions(&mut self, ctx: &mut Context) -> GameResult{

        let mut tree = DynamicBoundingVolumeTree::<Value>::new();
        for i in 0..self.enemies.len() {
            let enemy = self.enemies.get(i).unwrap();
            let minx = enemy.pos.x - ENEMY_WIDTH / 2f32;
            let miny = enemy.pos.y - ENEMY_WIDTH / 2f32;
            let maxx = enemy.pos.x + ENEMY_WIDTH / 2f32;
            let maxy = enemy.pos.y + ENEMY_WIDTH / 2f32;
            tree.insert(Value::new(aabb2(minx, miny, maxx, maxy), i as u16));
        }
        tree.tick();
        for bullet in &mut self.bullets{
            let minx = bullet.pos.x - BULLET_WIDTH / 2f32;
            let miny = bullet.pos.y - BULLET_WIDTH / 2f32;
            let maxx = bullet.pos.x + BULLET_WIDTH / 2f32;
            let maxy = bullet.pos.y + BULLET_WIDTH / 2f32;

            let bound = aabb2(minx, miny, maxx, maxy);
            let mut visitor = DiscreteVisitor::<Aabb2<f32>, Value>::new(&bound);
            let result = tree.query(&mut visitor);
            for enemy in result{
                let enemy = enemy.0;
                let xdist = enemy.aabb.max.x - ENEMY_WIDTH / 2f32 - bullet.pos.x;
                let ydist = enemy.aabb.max.y - ENEMY_WIDTH / 2f32 - bullet.pos.y;
                if sqrt((xdist * xdist + ydist * ydist) as f64) as f32 <= ENEMY_WIDTH / 2f32 {
                    if let Some(certain_enemy) = self.enemies.get_mut(enemy.index as usize) {
                        certain_enemy.health = 0;
                    }
                    bullet.health = 0;
                    self.dollars += 1;
                    for _ in 0..5{

                        //random in 20 degrees cone:
                        let randf = rand::random::<f32>() * PARTICLE_ANGLE - PARTICLE_ANGLE / 2f32;
                        let rot = bullet.rotation + randf;
                        let dir = vec_from_angle(-rot) * 5f32;
            
                        let new_particle = Entity{
                            entity_type: EntityTypes::Particle,
                            pos: bullet.pos + bullet.d,
                            d: dir,
                            image: graphics::Image::from_path(ctx, "/blood_particle.png").unwrap(),
                            health: PARTICLE_HEALTH,
                            rotation: rot,
                            frame: 0,
                            frame_time: 0,
                        };
            
                        self.particles.push(new_particle);
                    }
                }
            }
        }
        
        let minx = self.player.pos.x - PLAYER_WIDTH / 2f32;
        let miny = self.player.pos.y - PLAYER_WIDTH / 2f32;
        let maxx = self.player.pos.x + PLAYER_WIDTH / 2f32;
        let maxy = self.player.pos.y + PLAYER_WIDTH / 2f32;
        let bound = aabb2(minx, miny, maxx, maxy);
        let mut visitor = DiscreteVisitor::<Aabb2<f32>, Value>::new(&bound);
        let result = tree.query(&mut visitor);
        for enemy in result{
            let enemy = enemy.0;
            let xdist = enemy.aabb.max.x - ENEMY_WIDTH / 2f32 - self.player.pos.x;
            let ydist = enemy.aabb.max.y - ENEMY_WIDTH / 2f32 - self.player.pos.y;
            if sqrt((xdist * xdist + ydist * ydist) as f64) as f32 <= ENEMY_WIDTH / 2f32 {
                if let Some(certain_enemy) = self.enemies.get_mut(enemy.index as usize) {
                    certain_enemy.health = 0;
                }
                if self.player.health > 0{
                    self.player.health -= 5;
                }
            }
        }

        Ok(())
    }


    fn advance_frames(&mut self, entity: EntityTypes){
        match entity{
            EntityTypes::Player => {

                self.player.frame_time -= 1;

                if self.key_pressed.values().sum::<f32>() - self.key_pressed.get(&KeyCode::Space).unwrap() == 0f32 {
                    self.player.frame_time = PLAYER_FRAME_TIME;
                    if self.player.frame != 0 {
                        self.player.frame = 0;
                    }
                }

                if self.player.frame_time == 0 {
                    self.player.frame = (self.player.frame + 1) % 9;
                    self.player.frame_time = PLAYER_FRAME_TIME;
                }
            }
            EntityTypes::Enemy => {
                for enemy in &mut self.enemies {
                    if distance(&self.player, enemy) < FOG_DISTANCE{
                        if enemy.frame_time == 0{
                            enemy.frame = (enemy.frame + 1) % 4;
                            enemy.frame_time = ENEMY_FRAME_TIME;
                        }
                        enemy.frame_time -= 1;
                    }
                }
            }

            _ => (),
        }
    }

    fn draw_entity(&mut self, entity: EntityTypes, canvas: &mut graphics::Canvas, ctx: &ggez::Context){

        match entity{
            EntityTypes::Player => {
                let player_param = graphics::DrawParam::default()
                    .dest(Vec2::new(self.player.pos.x, self.player.pos.y))
                    .scale(Vec2::new(2f32, 1.5))
                    .rotation(self.player.rotation)
                    .offset(Vec2::new(0.5, 0.5));

                let gun_rot = self.player.rotation;
                let dir = vec_from_angle(-gun_rot);
                let gun_x = self.player.pos.x + dir.x * (PLAYER_HEIGHT + 20f32) / 2f32;
                let gun_y = self.player.pos.y + dir.y * (PLAYER_HEIGHT + 20f32) / 2f32;

                let gun_param = graphics::DrawParam::default()
                    .dest(Vec2::new(gun_x, gun_y))
                    .scale(Vec2::new(2f32, 1.5))
                    .rotation(gun_rot)
                    .offset(Vec2::new(0.5, 0.5));

                if self.player.frame_time == PLAYER_FRAME_TIME{
                    self.player.image = graphics::Image::from_path(ctx, format!("/pl{}.png", self.player.frame + 1)).unwrap();
                }
                let gun_nr: u8;
                match self.using_gun{
                    Guns::Pistol => { gun_nr = 1;}
                    Guns::MachineGun => { gun_nr = 2;}
                }
                canvas.draw(&self.player.image, player_param);
                canvas.draw(&graphics::Image::from_path(ctx, format!("/gun{}.png", gun_nr)).unwrap(), gun_param);
            }
            EntityTypes::Bullet => {
                let bullet_param = graphics::DrawParam::default()
                    .offset(Vec2::new(0.5, 0.5))
                    .scale(Vec2::new(3f32, 3f32));
                for bullet in &self.bullets {
                    if distance(&self.player, bullet) < FOG_DISTANCE {
                        canvas.draw(&bullet.image, bullet_param
                            .dest(Vec2::new(bullet.pos.x, bullet.pos.y))
                            .rotation(bullet.rotation)
                        );
                    }
                }
            }
            EntityTypes::Enemy => {
                let enemy_param = graphics::DrawParam::default()
                    .offset(Vec2::new(0.5, 0.5));
                for enemy in &mut self.enemies {
                    if distance(&self.player, enemy) < FOG_DISTANCE{
                        if enemy.frame_time == 0{
                            let frame_nr = enemy.frame + 1;
                            enemy.image = graphics::Image::from_path(ctx, format!("/enemy_frame{frame_nr}.png")).unwrap();
                        }
                        canvas.draw(&enemy.image, enemy_param
                            .dest(Vec2::new(enemy.pos.x, enemy.pos.y))
                            .rotation(enemy.rotation)
                        );
                    }
                }
            }
            EntityTypes::Particle => {
                let particle_param = graphics::DrawParam::default()
                    .offset(Vec2::new(0.5, 0.5));
                for particle in &self.particles {
                    if distance(&self.player, particle) < FOG_DISTANCE {
                        canvas.draw(&particle.image, particle_param
                            .dest(Vec2::new(particle.pos.x, particle.pos.y))
                            .rotation(particle.rotation)
                            .scale(Vec2::new(2f32, 2f32))
                            .color(graphics::Color::new(1f32, 0f32, 0f32, (particle.health as f32) / (PARTICLE_HEALTH as f32)))
                        );
                    }
                }
            }
        }
    }

    fn menu_guns(&mut self, canvas: &mut graphics::Canvas){
        let mut x_pos = 100f32;
        let y_pos = 600f32;
        for gun in &self.guns{
            let stat = gun.1;
            let mut str1 = "";
            let mut str2 = "";
            match stat{
                0 => {
                    str1 = "not bought";
                    str2 = "buy for 200 dollars";
                }
                1 => {
                    str1 = "ready for use";
                    str2 = "can switch to";
                }
                2 => {
                    str1 = "currently using";
                    str2 = "currently using";
                }
                _ => {}
            }

            let gun_nr: u8;
            match gun.0{
                Guns::Pistol => {
                    canvas.draw(&graphics::Text::new(format!("Pistol: {str1}")), 
                        ggez::graphics::DrawParam::default().dest(Vec2::new(x_pos, y_pos)).color(Color::YELLOW));
                    gun_nr = 1;
                }
                    Guns::MachineGun => {
                        canvas.draw(&graphics::Text::new(format!("Machine Gun: {str1}")), 
                            ggez::graphics::DrawParam::default().dest(Vec2::new(x_pos, y_pos)).color(Color::YELLOW));
                    gun_nr = 2;
                }
            }
            canvas.draw(&graphics::Text::new(format!("{str2} (key {gun_nr})")), 
                ggez::graphics::DrawParam::default().dest(Vec2::new(x_pos, y_pos + 25f32)).color(Color::YELLOW));
            x_pos += 250f32;
        }

    }
}

fn aabb2(minx: f32, miny: f32, maxx: f32, maxy: f32) -> Aabb2<f32> {
    Aabb2::new(Point2::new(minx, miny), Point2::new(maxx, maxy))
 }

#[derive(Clone)]
struct Value {
    pub aabb: Aabb2<f32>,
    fat_aabb: Aabb2<f32>,
    index: u16,
}

impl Value {
    pub fn new(aabb: Aabb2<f32>, pos: u16) -> Self {
        Self {
            fat_aabb : aabb,
            aabb,
            index: pos
        }
    }
}

impl TreeValue for Value {
    type Bound = Aabb2<f32>;

    fn bound(&self) -> &Aabb2<f32> {
        &self.aabb
    }

    fn get_bound_with_margin(&self) -> Aabb2<f32> {
        self.fat_aabb.clone()
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> ggez::GameResult {
        match self.state{
            State::Playing => {

                self.advance_frames(EntityTypes::Player);
                self.advance_frames(EntityTypes::Enemy);

                //egui
                //let egui_ctx = self.egui.ctx();
                //egui::Window::new("egui-window").show(&egui_ctx, |ui| {
		        //	ui.label("a very nice gui :3");
		        //	if ui.button("print \"hello world\"").clicked() {
		        //		println!("hello world");
		        //	}
		        //});
                
                self.handle_bounderies();
                
                self.player.pos.x = self.player.pos.x - self.key_pressed[&KeyCode::A] + self.key_pressed[&KeyCode::D];
                self.player.pos.y = self.player.pos.y - self.key_pressed[&KeyCode::W] + self.key_pressed[&KeyCode::S];
                
                //rotate player towards cursor
                self.player.rotation = atan2f(self.mouse_pos.y - self.player.pos.y, self.mouse_pos.x - self.player.pos.x) - PI / 2f32;
                
                //move bullets
                for bullet in &mut self.bullets{
                    bullet.health -= 1;
                    bullet.pos.x += bullet.d.x;
                    bullet.pos.y += bullet.d.y;
                }
            
                //move particles
                for particle in &mut self.particles {
                    particle.pos += particle.d;
                    particle.health -= 1;
                    particle.d.x = particle.d.x / 1.1f32;
                    particle.d.y = particle.d.y / 1.1f32;
                }
            
                //move enemies towards player
                for enemy in &mut self.enemies{
                    enemy.rotation = atan2f(self.player.pos.y - enemy.pos.y, self.player.pos.x - enemy.pos.x) - PI / 2f32;
                    enemy.pos.x += enemy.d.x;
                    enemy.pos.y += enemy.d.y;
                    let dir = vec_from_angle(-enemy.rotation);
                    enemy.d.x = dir.x * ENEMY_SPEED;
                    enemy.d.y = dir.y * ENEMY_SPEED;
                }
            
                self.handle_collisions(ctx)?;
            
                //clear bullets
                self.clear_entities();
            
                //reloading
                if self.reloading != 0 {
                    self.reloading -= 1;
                }
            
                //counting down towards new enemy
                if self.counter != 0 {
                    self.counter -= 1;
                }
            }
            State::Paused => {
                self.reloading = 179;
            }
            State::Unpausing => {
                self.reloading -= 1;
                if self.reloading == 0{
                    self.state = State::Playing;
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> ggez::GameResult {

        let mut canvas = graphics::Canvas::from_frame(ctx, Color::from_rgb(0,26,17));

        //if space is currently pressed, fire shot.
        if self.key_pressed[&KeyCode::Space] == 1f32 && self.reloading == 0 {
            self.fire_shot(ctx)?;
        }
        if self.counter == 0{
            self.spawn_enemy(ctx)?;
        }
        //draw particles
        self.draw_entity(EntityTypes::Particle, &mut canvas, ctx);
        //draw player
        self.draw_entity(EntityTypes::Player, &mut canvas, ctx);
        //draw bullets
        self.draw_entity(EntityTypes::Bullet, &mut canvas, ctx);
        //draw enemies
        self.draw_entity(EntityTypes::Enemy, &mut canvas, ctx);
        //draw BG
        canvas.draw(&self.bg, graphics::DrawParam::default()
            .offset(Vec2::new(0.5, 0.5))
            .dest(self.player.pos));
        //draw egui
        //let egui_param = graphics::DrawParam::default()
        //    .dest(Vec2::new(WIN_WIDTH, WIN_HEIGHT));
        //
        //canvas.draw(&self.egui, egui_param);
        match self.state{
            State::Paused => {
                let bg_param = graphics::DrawParam::default()
                                            .dest(Vec2::new(WIN_WIDTH / 2f32, WIN_HEIGHT / 2f32))
                                            .offset(Vec2::new(0.5, 0.5))
                                            .color(graphics::Color::new(55f32, 148f32, 110f32, 0.05));
                canvas.draw(&self.paused_bg, bg_param);
                self.menu_guns(&mut canvas);
            },
            State::Unpausing => {
                let bg_param = graphics::DrawParam::default()
                                            .dest(Vec2::new(WIN_WIDTH / 2f32, WIN_HEIGHT / 2f32))
                                            .offset(Vec2::new(0.5, 0.5))
                                            .color(graphics::Color::new(1f32, 0f32, 0f32, 0.05));
                canvas.draw(&graphics::Image::from_path(ctx, "/pause_bg.png").unwrap(), bg_param);
                //draw cursor
                let cursor_param = graphics::DrawParam::default()
                    .dest(self.mouse_pos)
                    .scale(Vec2::new(2.5f32, 2.5f32))
                    .offset(Vec2::new(0.5, 0.5));
                canvas.draw(&self.cursor, cursor_param);
                let left_secs = self.reloading / 60 + 1;
                let countdown_param = graphics::DrawParam::default()
                                                    .dest(Vec2::new(WIN_WIDTH / 2f32, WIN_HEIGHT / 2f32))
                                                    .offset(Vec2::new(0.5, 0.5));
                canvas.draw(&graphics::Image::from_path(ctx, format!("/countdown{}.png", left_secs)).unwrap(), countdown_param);
            },
            State::Playing => {
                self.draw_entity(EntityTypes::Player, &mut canvas, ctx);
                //draw cursor
                let cursor_param = graphics::DrawParam::default()
                    .dest(self.mouse_pos)
                    .scale(Vec2::new(2.5f32, 2.5f32))
                    .offset(Vec2::new(0.5, 0.5));
                canvas.draw(&self.cursor, cursor_param);
            }
        }
        //draw FPS & enemies & HP & dollars
        let fps = ctx.time.fps() as i16;
        canvas.draw(&graphics::Text::new(fps.to_string()),
            ggez::graphics::DrawParam::default().dest(Vec2::new(0f32, 0f32)).color(Color::YELLOW));
        canvas.draw(&graphics::Text::new(format!("enemies: {}", self.enemies.len())), 
            ggez::graphics::DrawParam::default().dest(Vec2::new(0f32, 25f32)).color(Color::YELLOW));
        canvas.draw(&graphics::Text::new(format!("HP: {}", self.player.health)), 
            ggez::graphics::DrawParam::default().dest(Vec2::new(0f32, 50f32)).color(Color::YELLOW));
        canvas.draw(&graphics::Text::new(format!("dollars: {}", self.dollars)), 
            ggez::graphics::DrawParam::default().dest(Vec2::new(0f32, 75f32)).color(Color::YELLOW));

        canvas.finish(ctx)?;

        Ok(())
    }


    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _: f32,
        _: f32,
    ) -> GameResult {
        //make player "look" at mouse position.
        self.mouse_pos.x = x;
        self.mouse_pos.y = y;
        Ok(())
    }

    fn key_down_event(&mut self, _: &mut Context, input: KeyInput, _: bool) -> GameResult {
        // if we press WAS or D, move accordingly
        match input.keycode {
            Some(KeyCode::W) | Some(KeyCode::A) | Some(KeyCode::S) | Some(KeyCode::D) => {
                let key = input.keycode.unwrap();
                *self.key_pressed.get_mut(&key).unwrap() = PX_MOVEMENT;
            },
            Some(KeyCode::Space) => {
                if self.reloading == 0 {
                    let key = input.keycode.unwrap();
                    *self.key_pressed.get_mut(&key).unwrap() = 1f32;
                }
            },
            Some(KeyCode::P) => {
                match self.state {
                    State::Playing => self.state = State::Paused,
                    State::Paused => self.state = State::Unpausing,
                    State::Unpausing => self.state = State::Paused,
                }
            }
            Some(KeyCode::Key1) | Some(KeyCode::Key2) => {
                if self.state == State::Paused{
                    let key = input.keycode.unwrap();
                    let new_gun: Guns;
                    match key{
                        KeyCode::Key1 => { new_gun = Guns::Pistol; }
                        _ => { new_gun = Guns::MachineGun; }
                    }
                    if self.guns[&new_gun] == 0{
                        if self.dollars >= 200{
                            self.dollars -= 200;
                            *self.guns.get_mut(&new_gun).unwrap() = 1;
                        }
                    }
                    else{
                        *self.guns.get_mut(&self.using_gun).unwrap() = 1;
                        self.using_gun = new_gun;
                        *self.guns.get_mut(&self.using_gun).unwrap() = 2;
                    }
                }
            }
            _ => (),
        }


        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        match input.keycode {
            Some(KeyCode::W) | Some(KeyCode::A) | Some(KeyCode::S) | Some(KeyCode::D) => {
                let key = input.keycode.unwrap();
                *self.key_pressed.get_mut(&key).unwrap() = 0f32;
            },
            Some(KeyCode::Space) => {
                    let key = input.keycode.unwrap();
                    *self.key_pressed.get_mut(&key).unwrap() = 0f32;
            },
            _ => (),
        }
        Ok(())
    }
}


fn main() -> ggez::GameResult {
    let cb = ggez::ContextBuilder::new("rect moving", "cstn")
        .resources_dir_name(r"Z:\informatica\projects\rust_noob\rect_practice\target\debug\resources")
        .window_mode(ggez::conf::WindowMode::default().dimensions(settings::WIN_WIDTH, settings::WIN_HEIGHT))
        .window_setup(ggez::conf::WindowSetup::default().title("An easy, good game."));

    // create a mutable reference to a `Context` and `EventsLoop
    let (mut ctx, event_loop) = cb.build()?;

    // Make a mutable reference to `MainState`
    let main_state = MainState::new(&mut ctx)?;

    // Start the game
    ggez::event::run(ctx, event_loop, main_state)
}