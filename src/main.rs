use bevy::app::App;
use bevy::color::palettes::css::{ORANGE, RED};
use bevy::prelude::*;
use rand;
use rand::Rng;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Mouse::default())
        .add_systems(
            Update,
            (
                calculate_mouse_position,
                spawn_bubble,
                move_bubbles,
                draw_bubbles,
                despawn_bubbles,
                move_ship,
                draw_ship,
                spawn_enemies,
                draw_enemies,
                check_bubble_enemy_collision,
                update_bubble_lifetime,
                handle_ship_border,
                check_game_over,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnEnter(GameState::GameOver), spawn_game_over_ui)
        .add_systems(OnExit(GameState::Playing), cleanup_gameplay)
        .add_systems(OnEnter(GameState::Playing), setup_game_round)
        .add_systems(
            Update,
            handle_replay_button.run_if(in_state(GameState::GameOver)),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

#[derive(Component, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct Bubble {
    color: Color,
    size: f32,
    lifetime: Timer,
}

#[derive(Component)]
#[require(Transform, Velocity)]
struct Ship {
    health: f32,
}

#[derive(Component)]
struct Enemy {
    health: f32,
    variant: EnemyVariant,
}

#[derive(Component)]
enum EnemyVariant {
    Floater,
    Seeker,
    // Add more variants as we implement them
}

fn random_pastel_color() -> Color {
    let mut rng = rand::thread_rng();
    Color::hsl(
        rng.gen_range(0.0..360.0), // Random hue
        0.7,                       // High saturation
        0.8,                       // High lightness for pastel
    )
}

fn spawn_bubble(
    mut commands: Commands,
    ship_query: Query<&Transform, With<Ship>>,
    mut ship_velocity: Query<&mut Velocity, With<Ship>>,
    mouse: Res<Mouse>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    if let (Ok(ship_transform), Ok(mut ship_vel)) =
        (ship_query.get_single(), ship_velocity.get_single_mut())
    {
        if mouse_button.pressed(MouseButton::Left) {
            let ship_pos = ship_transform.translation.truncate();
            let mut rng = rand::thread_rng();

            // Calculate direction to mouse with some randomness
            let to_mouse = (mouse.position - ship_pos).normalize();
            let random_angle = rng.gen_range(-0.3..0.3);
            let direction = Vec2::new(
                to_mouse.x * (random_angle as f32).cos() - to_mouse.y * (random_angle as f32).sin(),
                to_mouse.x * (random_angle as f32).sin() + to_mouse.y * (random_angle as f32).cos(),
            );
            let speed = rng.gen_range(100.0..200.0);

            // Apply recoil to ship
            let recoil_force = 5.0;
            ship_vel.0 -= direction * recoil_force;

            commands.spawn((
                Bubble {
                    color: random_pastel_color(),
                    size: rng.gen_range(5.0..15.0),
                    lifetime: Timer::from_seconds(rng.gen_range(1.0..2.0), TimerMode::Once),
                },
                Transform::from_xyz(ship_pos.x, ship_pos.y, 0.0),
                Velocity(direction * speed),
                GameplayObject,
            ));
        }
    }
}

fn move_bubbles(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation += velocity.0.extend(0.0) * time.delta_secs()
    }
}

#[derive(Resource, Debug, Default)]
struct Mouse {
    position: Vec2,
}

fn calculate_mouse_position(
    camera_query: Query<(&GlobalTransform, &Camera)>,
    window_query: Query<&Window>,
    mut mouse: ResMut<Mouse>,
) {
    let (camera_transform, camera) = camera_query.single();
    let window = window_query.single();

    let position = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
        .unwrap_or_default();

    mouse.position = position;
}

fn draw_bubbles(mut gizmos: Gizmos, query: Query<(&Transform, &Bubble)>) {
    for (transform, bubble) in &query {
        let pos = transform.translation.truncate();
        let radius = bubble.size;

        // Outer glow
        gizmos.circle_2d(pos, radius + 2.0, bubble.color.with_alpha(0.2));

        // Main bubble outline
        gizmos.circle_2d(pos, radius, bubble.color.with_alpha(0.8));

        // Inner highlight
        gizmos.circle_2d(
            pos + Vec2::new(-radius * 0.2, radius * 0.2),
            radius * 0.4,
            Color::WHITE.with_alpha(0.3),
        );

        // Shine detail
        gizmos.circle_2d(
            pos + Vec2::new(-radius * 0.1, radius * 0.1),
            radius * 0.2,
            Color::WHITE.with_alpha(0.5),
        );
    }
}

fn despawn_bubbles(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Bubble>>,
    window_query: Query<&Window>,
) {
    let window = window_query.single();
    let half_width = window.width() / 2.0;
    let half_height = window.height() / 2.0;

    for (entity, transform) in &query {
        let pos = transform.translation;
        if pos.x < -half_width || pos.x > half_width || pos.y < -half_height || pos.y > half_height
        {
            commands.entity(entity).despawn();
        }
    }
}

fn move_ship(
    mut query: Query<(&mut Transform, &mut Velocity), With<Ship>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    window_query: Query<&Window>,
) {
    if let Ok((mut transform, mut velocity)) = query.get_single_mut() {
        let window = window_query.single();
        let half_width = window.width() / 2.0;
        let half_height = window.height() / 2.0;

        let mut acceleration = Vec2::ZERO;
        let acceleration_rate = 1000.0;
        let max_speed = 300.0;
        let friction = 0.98;

        if keyboard.pressed(KeyCode::KeyW) {
            acceleration.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            acceleration.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            acceleration.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            acceleration.x += 1.0;
        }

        let dt = time.delta_secs();

        if acceleration != Vec2::ZERO {
            acceleration = acceleration.normalize() * acceleration_rate * dt;
            velocity.0 += acceleration;
        }

        // Apply friction
        velocity.0 *= friction;

        // Clamp maximum speed
        if velocity.0.length() > max_speed {
            velocity.0 = velocity.0.normalize() * max_speed;
        }

        transform.translation += velocity.0.extend(0.0) * dt;

        // Wrap position around screen edges
        if transform.translation.x > half_width {
            transform.translation.x = -half_width;
        } else if transform.translation.x < -half_width {
            transform.translation.x = half_width;
        }

        if transform.translation.y > half_height {
            transform.translation.y = -half_height;
        } else if transform.translation.y < -half_height {
            transform.translation.y = half_height;
        }
    }
}

fn draw_ship(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &Ship)>,
    mouse: Res<Mouse>,
    window_query: Query<&Window>,
) {
    let window = window_query.single();
    let border_width = 50.0;

    // Draw danger border
    gizmos.rect_2d(
        Vec2::ZERO, // center position
        Vec2::new(
            window.width() - border_width,
            window.height() - border_width,
        ), // size
        Color::srgba(1.0, 0.0, 0.0, 0.2), // color
    );

    if let Ok((transform, ship)) = query.get_single() {
        let pos = transform.translation.truncate();

        // Calculate ship color based on health (100 -> white, 0 -> dark red)
        let health_factor = (ship.health / 100.0).clamp(0.0, 1.0);
        let ship_color = Color::srgb(
            1.0,           // Red stays at 1.0
            health_factor, // Green fades with health
            health_factor, // Blue fades with health
        );

        // Draw ship circle with health color
        gizmos.circle_2d(pos, 15.0, ship_color);

        // Draw aim line with same color
        let to_mouse = (mouse.position - pos).normalize();
        let rect_length = 20.0;
        let rect_center = pos + to_mouse * 15.0;

        gizmos.line_2d(
            rect_center - to_mouse * rect_length / 2.0,
            rect_center + to_mouse * rect_length / 2.0,
            ship_color,
        );
    }
}

fn spawn_enemies(mut commands: Commands, time: Res<Time>, window_query: Query<&Window>) {
    let window = window_query.single();
    let mut rng = rand::thread_rng();

    // Spawn every few seconds
    if time.elapsed_secs() % 3.0 < time.delta_secs() {
        let x = rng.gen_range(-window.width() / 2.0..window.width() / 2.0);
        let y = rng.gen_range(-window.height() / 2.0..window.height() / 2.0);

        commands.spawn((
            Enemy {
                health: 100.0,
                variant: EnemyVariant::Floater,
            },
            Transform::from_xyz(x, y, 0.0),
            Velocity(Vec2::new(
                rng.gen_range(-50.0..50.0),
                rng.gen_range(-50.0..50.0),
            )),
            GameplayObject,
        ));
    }
}

fn draw_enemies(mut gizmos: Gizmos, query: Query<(&Transform, &Enemy)>) {
    for (transform, enemy) in &query {
        let pos = transform.translation.truncate();
        match enemy.variant {
            EnemyVariant::Floater => {
                // Draw red circle for floaters
                gizmos.circle_2d(pos, 20.0, RED);
            }
            EnemyVariant::Seeker => {
                // Draw orange triangle for seekers
                let points = [
                    pos + Vec2::new(0.0, 20.0),
                    pos + Vec2::new(-17.3, -10.0),
                    pos + Vec2::new(17.3, -10.0),
                ];
                gizmos.linestrip_2d(points, ORANGE);
            }
        }
    }
}

fn check_bubble_enemy_collision(
    mut commands: Commands,
    bubble_query: Query<(Entity, &Transform), With<Bubble>>,
    mut enemy_query: Query<(Entity, &Transform, &mut Enemy)>,
) {
    let mut destroyed_enemies: Vec<Entity> = Vec::new();
    let mut destroyed_bubbles: Vec<Entity> = Vec::new();

    for (bubble_entity, bubble_transform) in bubble_query.iter() {
        if destroyed_bubbles.contains(&bubble_entity) {
            continue;
        }

        let bubble_pos = bubble_transform.translation.truncate();

        for (enemy_entity, enemy_transform, mut enemy) in enemy_query.iter_mut() {
            if destroyed_enemies.contains(&enemy_entity) {
                continue;
            }

            let enemy_pos = enemy_transform.translation.truncate();

            if bubble_pos.distance(enemy_pos) < 30.0 {
                enemy.health -= 25.0;
                destroyed_bubbles.push(bubble_entity);

                if enemy.health <= 0.0 {
                    destroyed_enemies.push(enemy_entity);
                }
                break; // Bubble can only hit one enemy
            }
        }
    }

    // Despawn all at once after collision checks
    for entity in destroyed_bubbles {
        commands.entity(entity).despawn();
    }
    for entity in destroyed_enemies {
        commands.entity(entity).despawn();
    }
}

fn update_bubble_lifetime(
    mut commands: Commands,
    mut bubbles: Query<(Entity, &mut Bubble)>,
    time: Res<Time>,
) {
    for (entity, mut bubble) in &mut bubbles {
        bubble.lifetime.tick(time.delta());
        if bubble.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_ship_border(
    mut ship_query: Query<(&mut Ship, &Transform, &mut Velocity)>,
    window_query: Query<&Window>,
) {
    if let Ok((mut ship, transform, mut velocity)) = ship_query.get_single_mut() {
        let window = window_query.single();
        let impact_damage = 10.0; // Fixed damage on impact
        let bounce_force = 500.0;
        let border_width = 50.0;

        let pos = transform.translation;
        let half_width = window.width() / 2.0 - border_width;
        let half_height = window.height() / 2.0 - border_width;

        // Check if ship just entered the border zone
        if pos.x.abs() > half_width || pos.y.abs() > half_height {
            // Only apply damage if ship is moving towards the border
            let to_center = -pos.truncate().normalize();
            if velocity.0.dot(to_center) < 0.0 {
                ship.health -= impact_damage;
                velocity.0 += to_center * bounce_force;
            }
        }
    }
}

fn check_game_over(ship_query: Query<&Ship>, mut next_state: ResMut<NextState<GameState>>) {
    if let Ok(ship) = ship_query.get_single() {
        if ship.health <= 0.0 {
            next_state.set(GameState::GameOver);
        }
    }
}

fn spawn_game_over_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            // Game Over Text
            parent.spawn(Text::new("Game Over"));

            // Replay Button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(50.0),
                        margin: UiRect::all(Val::Px(20.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    ReplayButton,
                ))
                .with_children(|parent| {
                    parent.spawn(Text::new("Replay"));
                });
        });
}

#[derive(Component)]
struct ReplayButton;

fn handle_replay_button(
    mut next_state: ResMut<NextState<GameState>>,
    mut interaction_query: Query<
        (&Interaction, &Parent),
        (Changed<Interaction>, With<ReplayButton>),
    >,
    mut commands: Commands,
) {
    for (interaction, parent) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Playing);
            commands.entity(parent.get()).despawn_recursive();
        }
    }
}

// Add marker component for gameplay entities
#[derive(Component)]
struct GameplayObject;

// Add cleanup system
fn cleanup_gameplay(mut commands: Commands, query: Query<Entity, With<GameplayObject>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// Add this new system
fn setup_game_round(mut commands: Commands) {
    commands.spawn((
        Ship { health: 100.0 },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Velocity::default(),
        GameplayObject,
    ));
}
