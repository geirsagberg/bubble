use bevy::app::App;
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
        transform.translation += velocity.0.extend(0.0) * time.delta().as_secs_f32();
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

    let dt = time.delta().as_secs_f32();

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
