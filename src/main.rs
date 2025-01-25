use bevy::app::App;
use bevy::color::palettes::css::{ORANGE, RED};
use bevy::prelude::*;
use rand;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands.spawn((
        Ship,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Velocity(Vec2::ZERO),
    ));
}

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Bubble {
    color: Color,
    size: f32,
}

#[derive(Component)]
struct Ship;

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
    if mouse_button.pressed(MouseButton::Left) {
        let ship_transform = ship_query.single();
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
        let mut ship_vel = ship_velocity.single_mut();
        ship_vel.0 -= direction * recoil_force;

        commands.spawn((
            Bubble {
                color: random_pastel_color(),
                size: rng.gen_range(5.0..15.0),
            },
            Transform::from_xyz(ship_pos.x, ship_pos.y, 0.0),
            Velocity(direction * speed),
        ));
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
    let (mut transform, mut velocity) = query.single_mut();
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

fn draw_ship(mut gizmos: Gizmos, query: Query<&Transform, With<Ship>>, mouse: Res<Mouse>) {
    let transform = query.single();
    let pos = transform.translation.truncate();

    // Draw ship circle
    gizmos.circle_2d(pos, 15.0, Color::WHITE);

    // Calculate direction to mouse
    let to_mouse = (mouse.position - pos).normalize();

    // Calculate rectangle points
    let rect_length = 20.0;
    let rect_center = pos + to_mouse * 15.0; // Position rectangle at edge of circle

    // Draw rectangle pointing towards mouse
    gizmos.line_2d(
        rect_center - to_mouse * rect_length / 2.0,
        rect_center + to_mouse * rect_length / 2.0,
        Color::WHITE,
    );
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
    for (bubble_entity, bubble_transform) in bubble_query.iter() {
        let bubble_pos = bubble_transform.translation.truncate();

        for (enemy_entity, enemy_transform, mut enemy) in enemy_query.iter_mut() {
            let enemy_pos = enemy_transform.translation.truncate();

            if bubble_pos.distance(enemy_pos) < 30.0 {
                // Adjust collision radius as needed
                enemy.health -= 25.0; // Damage from bubble
                commands.entity(bubble_entity).despawn();

                if enemy.health <= 0.0 {
                    commands.entity(enemy_entity).despawn();
                }

                break; // Bubble can only hit one enemy
            }
        }
    }
}
