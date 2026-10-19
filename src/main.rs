mod icon;

use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::collections::VecDeque;

const BASE_SPEED: f64 = 0.12;
const MIN_CELL: f32 = 10.0;
const MAX_CELL: f32 = 60.0;
const EAT_ANIM: f64 = 0.5;

const BG_TOP: Color = Color::new(0.05, 0.08, 0.12, 1.0);
const BG_BOTTOM: Color = Color::new(0.10, 0.16, 0.22, 1.0);
const ACCENT: Color = Color::new(0.24, 0.86, 0.52, 1.0);
const FOOD_RED: Color = Color::new(1.0, 0.28, 0.34, 1.0);

const OPT_SPEED: usize = 0;
const OPT_FRUITS: usize = 1;
const OPT_SELF: usize = 2;
const OPT_WALLS: usize = 3;
const OPT_GRID: usize = 4;
const OPT_TRAIL: usize = 5;
const OPT_AUTOPLAY: usize = 6;

const SPEED_MULT: [f32; 4] = [0.7, 1.0, 1.5, 2.2];
const FRUIT_LABELS: &[&str] = &["1", "2", "3", "5", "10", "25", "50", "10%", "5%", "2%", "1%"];
const FRUIT_COUNTS: [usize; 7] = [1, 2, 3, 5, 10, 25, 50];
const FRUIT_PCTS: [f32; 4] = [10.0, 5.0, 2.0, 1.0];

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    fn to_vec(self) -> Vec2 {
        match self {
            Direction::Up => vec2(0.0, -1.0),
            Direction::Down => vec2(0.0, 1.0),
            Direction::Left => vec2(-1.0, 0.0),
            Direction::Right => vec2(1.0, 0.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Start,
    Playing,
    Over,
    Win,
}

struct EatenFruit {
    pos: Vec2,
    start: f64,
}

struct GameOption {
    name: &'static str,
    values: &'static [&'static str],
    idx: usize,
}

fn default_options() -> Vec<GameOption> {
    vec![
        GameOption { name: "Speed", values: &["Slow", "Normal", "Fast", "Turbo"], idx: 1 },
        GameOption { name: "Fruits", values: FRUIT_LABELS, idx: 0 },
        GameOption { name: "Self Collision", values: &["On", "Off"], idx: 0 },
        GameOption { name: "Walls", values: &["Solid", "Wrap"], idx: 0 },
        GameOption { name: "Grid Lines", values: &["On", "Off"], idx: 0 },
        GameOption { name: "Trail Mod", values: &["Off", "On"], idx: 0 },
        GameOption { name: "Autoplay", values: &["Off", "On"], idx: 0 },
    ]
}

fn start_snake() -> Vec<Vec2> {
    vec![vec2(9.0, 9.0), vec2(8.0, 9.0), vec2(7.0, 9.0)]
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Snake".to_owned(),
        window_width: 800,
        window_height: 600,
        window_resizable: true,
        icon: Some(Icon {
            small: icon::SMALL,
            medium: icon::MEDIUM,
            big: icon::BIG,
        }),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut cell: f32 = 30.0;
    let mut snake = start_snake();
    let mut prev_snake = snake.clone();
    let mut dir = Direction::Right;
    let mut next_dir = Direction::Right;
    let mut score = 0;
    let mut phase = Phase::Start;
    let mut last_tick = get_time();
    let mut fullscreen = false;
    let mut options = default_options();
    let mut show_options = false;
    let mut option_cursor = 0usize;
    let mut foods: Vec<Vec2> = Vec::new();
    let mut pending_fruits: usize;
    let mut eaten: Vec<EatenFruit> = Vec::new();
    let mut trail: VecDeque<(Vec2, f64)> = VecDeque::new();
    let mut recent_heads: VecDeque<Vec2> = VecDeque::new();
    let mut wander = 0u32;

    let (mut grid_w, mut grid_h) = grid_size(cell);
    pending_fruits = sync_fruits(&mut foods, &snake, grid_w, grid_h, fruit_target(options[OPT_FRUITS].idx, free_cells(grid_w, grid_h, &snake)));

    loop {
        set_default_camera();
        clear_background(BG_TOP);

        let speed_mult = SPEED_MULT[options[OPT_SPEED].idx];
        let interval = BASE_SPEED / speed_mult as f64;
        let fruit_count = fruit_target(options[OPT_FRUITS].idx, free_cells(grid_w, grid_h, &snake));
        let self_collision_on = options[OPT_SELF].idx == 0;
        let walls_solid = options[OPT_WALLS].idx == 0;
        let grid_on = options[OPT_GRID].idx == 0;
        let trail_on = options[OPT_TRAIL].idx == 1;
        let autoplay_on = options[OPT_AUTOPLAY].idx == 1;

        if is_key_pressed(KeyCode::KpAdd) || is_key_pressed(KeyCode::Equal) {
            cell = (cell + 5.0).clamp(MIN_CELL, MAX_CELL);
        }
        if is_key_pressed(KeyCode::KpSubtract) || is_key_pressed(KeyCode::Minus) {
            cell = (cell - 5.0).clamp(MIN_CELL, MAX_CELL);
        }
        if is_key_pressed(KeyCode::F) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }

        let (new_w, new_h) = grid_size(cell);
        if new_w != grid_w || new_h != grid_h {
            grid_w = new_w;
            grid_h = new_h;
            if walls_solid && snake.iter().any(|s| s.x >= grid_w as f32 || s.y >= grid_h as f32) {
                phase = Phase::Over;
            }
            foods.retain(|f| f.x < grid_w as f32 && f.y < grid_h as f32);
            pending_fruits = sync_fruits(&mut foods, &snake, grid_w, grid_h, fruit_count);
            eaten.retain(|e| e.pos.x < grid_w as f32 && e.pos.y < grid_h as f32);
        }

        if is_key_pressed(KeyCode::O) {
            show_options = !show_options;
        }
        if is_key_pressed(KeyCode::P) {
            let opt = &mut options[OPT_AUTOPLAY];
            opt.idx = if opt.idx == 0 { 1 } else { 0 };
        }

        if show_options {
            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                option_cursor = (option_cursor + options.len() - 1) % options.len();
            }
            if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                option_cursor = (option_cursor + 1) % options.len();
            }

            let (_, rows, close) = options_layout(options.len());
            let mpos = Vec2::from(mouse_position());
            for (i, r) in rows.iter().enumerate() {
                if r.contains(mpos) {
                    option_cursor = i;
                }
            }

            let mut advanced = is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D);
            let mut reversed = is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A);

            if is_mouse_button_pressed(MouseButton::Left) {
                if close.contains(mpos) {
                    show_options = false;
                } else if rows[option_cursor].contains(mpos) {
                    advanced = true;
                }
            }
            if is_mouse_button_pressed(MouseButton::Right) && rows[option_cursor].contains(mpos) {
                reversed = true;
            }

            if advanced {
                change_option(&mut options, option_cursor, 1);
                if option_cursor == OPT_FRUITS {
                    pending_fruits = sync_fruits(&mut foods, &snake, grid_w, grid_h, fruit_target(options[OPT_FRUITS].idx, free_cells(grid_w, grid_h, &snake)));
                }
            }
            if reversed {
                change_option(&mut options, option_cursor, -1);
                if option_cursor == OPT_FRUITS {
                    pending_fruits = sync_fruits(&mut foods, &snake, grid_w, grid_h, fruit_target(options[OPT_FRUITS].idx, free_cells(grid_w, grid_h, &snake)));
                }
            }

            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Space) {
                show_options = false;
            }
        } else {
            if is_key_pressed(KeyCode::W) || is_key_pressed(KeyCode::Up) {
                if dir != Direction::Down {
                    next_dir = Direction::Up;
                }
            }
            if is_key_pressed(KeyCode::S) || is_key_pressed(KeyCode::Down) {
                if dir != Direction::Up {
                    next_dir = Direction::Down;
                }
            }
            if is_key_pressed(KeyCode::A) || is_key_pressed(KeyCode::Left) {
                if dir != Direction::Right {
                    next_dir = Direction::Left;
                }
            }
            if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::Right) {
                if dir != Direction::Left {
                    next_dir = Direction::Right;
                }
            }

            match phase {
                Phase::Start => {
                    if is_key_pressed(KeyCode::Space) {
                        phase = Phase::Playing;
                        last_tick = get_time();
                    }
                }
                Phase::Over | Phase::Win => {
                    if is_key_pressed(KeyCode::Space) {
                        snake = start_snake();
                        prev_snake = snake.clone();
                        dir = Direction::Right;
                        next_dir = Direction::Right;
                        eaten.clear();
                        trail.clear();
                        foods.clear();
                        recent_heads.clear();
                        pending_fruits = sync_fruits(&mut foods, &snake, grid_w, grid_h, fruit_count);
                        score = 0;
                        phase = Phase::Playing;
                        last_tick = get_time();
                    }
                }
                Phase::Playing => {
                    let now = get_time();
                    let max_ticks: u32 = if autoplay_on { 10 } else { 1 };
                    let mut ticks = 0u32;
                    while (autoplay_on || now - last_tick >= interval)
                        && ticks < max_ticks
                        && phase == Phase::Playing
                    {
                        prev_snake = snake.clone();
                        last_tick = now;
                        if autoplay_on {
                            if recent_heads.contains(&snake[0]) {
                                wander = 8;
                            }
                            next_dir = autoplay_dir(&snake, &foods, grid_w, grid_h, walls_solid, self_collision_on, dir, wander > 0);
                            if wander > 0 {
                                wander -= 1;
                            }
                            recent_heads.push_back(snake[0]);
                            while recent_heads.len() > 24 {
                                recent_heads.pop_front();
                            }
                        } else {
                            recent_heads.clear();
                            wander = 0;
                        }
                        dir = next_dir;
                        let head = snake[0];
                        let mut new_head = head + dir.to_vec();
                        if !walls_solid {
                            new_head.x = new_head.x.rem_euclid(grid_w.max(1) as f32);
                            new_head.y = new_head.y.rem_euclid(grid_h.max(1) as f32);
                        }
                        snake.insert(0, new_head);

                        let ate = if let Some(i) = foods.iter().position(|&f| f == new_head) {
                            score += 1;
                            eaten.push(EatenFruit { pos: new_head, start: now });
                            match spawn_food(&snake, &foods, grid_w, grid_h) {
                                Some(c) => foods[i] = c,
                                None => {
                                    foods.remove(i);
                                    pending_fruits += 1;
                                }
                            }
                            true
                        } else {
                            false
                        };

                        if !ate {
                            snake.pop();
                        }

                        if snake.len() >= grid_w.saturating_mul(grid_h) {
                            phase = Phase::Win;
                            break;
                        }

                        if pending_fruits > 0 {
                            if let Some(c) = spawn_food(&snake, &foods, grid_w, grid_h) {
                                foods.push(c);
                                pending_fruits -= 1;
                            }
                        }

                        let hit_wall = walls_solid
                            && (new_head.x < 0.0
                                || new_head.y < 0.0
                                || new_head.x >= grid_w as f32
                                || new_head.y >= grid_h as f32);
                        let hit_self = self_collision_on && snake[1..].contains(&new_head);
                        if hit_wall || hit_self {
                            phase = Phase::Over;
                        }
                        ticks += 1;
                    }
                }
            }
        }

        let now = get_time();
        let t = match phase {
            Phase::Over | Phase::Win => 1.0,
            Phase::Start => 0.0,
            Phase::Playing => ((now - last_tick) / interval).min(1.0) as f32,
        };

        if phase == Phase::Playing {
            if trail_on {
                let hp = body_pos(&snake, &prev_snake, 0, t, !walls_solid, grid_w, grid_h);
                trail.push_back((hp, now));
                while trail.front().is_some_and(|&(_, t0)| now - t0 > 0.9) {
                    trail.pop_front();
                }
            } else {
                trail.clear();
            }
        }

        eaten.retain(|e| now - e.start < EAT_ANIM);
        draw_bg(grid_w, grid_h, cell, grid_on);
        draw_trail(&trail, cell, now);
        draw_foods(&foods, cell, now);
        draw_snake(&snake, &prev_snake, dir, t, cell, !walls_solid, grid_w, grid_h);
        draw_eaten(&eaten, cell, now);
        draw_border(grid_w, grid_h, cell, walls_solid, now);
        match phase {
            Phase::Start => draw_start_screen(score),
            Phase::Over => draw_game_over(score),
            Phase::Win => draw_win_screen(score),
            Phase::Playing => {
                let speed_name = options[OPT_SPEED].values[options[OPT_SPEED].idx];
                draw_hud(score, speed_name, autoplay_on);
            }
        }
        if show_options {
            draw_options(&options, option_cursor);
        }

        next_frame().await;
    }
}

fn grid_size(cell: f32) -> (usize, usize) {
    (
        (screen_width() / cell).floor() as usize,
        (screen_height() / cell).floor() as usize,
    )
}

fn change_option(options: &mut [GameOption], cursor: usize, delta: i32) {
    let opt = &mut options[cursor];
    opt.idx = (opt.idx as i32 + delta).rem_euclid(opt.values.len() as i32) as usize;
}

fn free_cells(grid_w: usize, grid_h: usize, snake: &[Vec2]) -> usize {
    grid_w.saturating_mul(grid_h).saturating_sub(snake.len())
}

fn fruit_target(idx: usize, free_cells: usize) -> usize {
    if free_cells == 0 {
        return 0;
    }
    if idx < FRUIT_COUNTS.len() {
        let count = FRUIT_COUNTS[idx];
        count.min(free_cells / 10).min(free_cells).max(1)
    } else {
        let pct = FRUIT_PCTS[idx - FRUIT_COUNTS.len()];
        let n = ((pct / 100.0) * free_cells as f32).round() as usize;
        n.clamp(1, free_cells)
    }
}

fn sync_fruits(foods: &mut Vec<Vec2>, snake: &[Vec2], grid_w: usize, grid_h: usize, count: usize) -> usize {
    while foods.len() > count {
        foods.pop();
    }
    let mut pending = 0;
    while foods.len() < count {
        match spawn_food(snake, foods, grid_w, grid_h) {
            Some(c) => foods.push(c),
            None => {
                pending = count - foods.len();
                break;
            }
        }
    }
    pending
}

fn spawn_food(snake: &[Vec2], foods: &[Vec2], grid_w: usize, grid_h: usize) -> Option<Vec2> {
    for _ in 0..2000 {
        let c = vec2(
            gen_range(0, grid_w.max(1) as i32) as f32,
            gen_range(0, grid_h.max(1) as i32) as f32,
        );
        if !snake.contains(&c) && !foods.contains(&c) {
            return Some(c);
        }
    }
    None
}

fn options_layout(rows: usize) -> (Rect, Vec<Rect>, Rect) {
    let w = screen_width();
    let h = screen_height();
    let row_h = 46.0;
    let panel_w = 460.0;
    let panel_h = 64.0 + rows as f32 * row_h + 66.0;
    let x = (w - panel_w) / 2.0;
    let y = (h - panel_h) / 2.0 - 10.0;
    let panel = Rect::new(x, y, panel_w, panel_h);
    let mut row_rects = Vec::new();
    for i in 0..rows {
        row_rects.push(Rect::new(x + 14.0, y + 58.0 + i as f32 * row_h, panel_w - 28.0, row_h - 10.0));
    }
    let close = Rect::new(x + panel_w / 2.0 - 60.0, y + panel_h - 56.0, 120.0, 40.0);
    (panel, row_rects, close)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        lerp(a.r, b.r, t),
        lerp(a.g, b.g, t),
        lerp(a.b, b.b, t),
        lerp(a.a, b.a, t),
    )
}

fn draw_rounded_rect(rect: Rect, radius: f32, color: Color) {
    let r = radius.min(rect.w * 0.5).min(rect.h * 0.5);
    draw_rectangle(rect.x + r, rect.y, rect.w - r * 2.0, rect.h, color);
    draw_rectangle(rect.x, rect.y + r, rect.w, rect.h - r * 2.0, color);
    draw_circle(rect.x + r, rect.y + r, r, color);
    draw_circle(rect.x + rect.w - r, rect.y + r, r, color);
    draw_circle(rect.x + r, rect.y + rect.h - r, r, color);
    draw_circle(rect.x + rect.w - r, rect.y + rect.h - r, r, color);
}

fn draw_bg(grid_w: usize, grid_h: usize, cell: f32, grid_on: bool) {
    let h = screen_height();
    let strips = 40;
    for i in 0..strips {
        let y = h * i as f32 / strips as f32;
        draw_rectangle(0.0, y, screen_width(), h / strips as f32 + 1.0, lerp_color(BG_TOP, BG_BOTTOM, i as f32 / strips as f32));
    }

    if grid_on {
        let line = Color::new(1.0, 1.0, 1.0, 0.045);
        for x in 1..grid_w {
            draw_line(x as f32 * cell, 0.0, x as f32 * cell, grid_h as f32 * cell, 1.0, line);
        }
        for y in 1..grid_h {
            draw_line(0.0, y as f32 * cell, grid_w as f32 * cell, y as f32 * cell, 1.0, line);
        }
    }
}

fn draw_border(grid_w: usize, grid_h: usize, cell: f32, walls_solid: bool, now: f64) {
    let w = grid_w as f32 * cell;
    let h = grid_h as f32 * cell;

    if walls_solid {
        let pulse = 0.5 + 0.5 * (now * 2.5).sin() as f32;
        for k in 1..=3 {
            let t = k as f32 * 4.0;
            draw_rectangle_lines(-t, -t, w + t * 2.0, h + t * 2.0, 3.0, Color::new(1.0, 0.35, 0.12, 0.10 / k as f32));
        }
        draw_rectangle_lines(0.0, 0.0, w, h, 5.0, Color::new(1.0, 0.45, 0.20, 0.65 + 0.30 * pulse));
        draw_rectangle_lines(2.0, 2.0, w - 4.0, h - 4.0, 1.0, Color::new(1.0, 0.85, 0.5, 0.35 * pulse));
    } else {
        let dash = 12.0;
        let gap = 9.0;
        let step = dash + gap;
        let offset = ((now * 46.0) as f32).rem_euclid(step);
        let col = Color::new(0.35, 0.82, 1.0, 0.55);
        draw_dashes(0.0, 0.0, w, 0.0, dash, gap, offset, 3.0, col);
        draw_dashes(0.0, h, w, h, dash, gap, offset, 3.0, col);
        draw_dashes(0.0, 0.0, 0.0, h, dash, gap, offset, 3.0, col);
        draw_dashes(w, 0.0, w, h, dash, gap, offset, 3.0, col);
        draw_rectangle_lines(0.0, 0.0, w, h, 1.5, Color::new(0.35, 0.82, 1.0, 0.22));
        for k in 1..=2 {
            let t = k as f32 * 5.0;
            draw_rectangle_lines(-t, -t, w + t * 2.0, h + t * 2.0, 2.0, Color::new(0.35, 0.82, 1.0, 0.06 / k as f32));
        }
    }
}

fn draw_dashes(x0: f32, y0: f32, x1: f32, y1: f32, dash: f32, gap: f32, offset: f32, width: f32, color: Color) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let step = dash + gap;
    let mut t = -step + offset;
    while t < len {
        let s = (t + dash).min(len);
        if s > 0.0 && t < len {
            let a = t.max(0.0);
            draw_line(x0 + ux * a, y0 + uy * a, x0 + ux * s, y0 + uy * s, width, color);
        }
        t += step;
    }
}

fn draw_foods(foods: &[Vec2], cell: f32, now: f64) {
    for food in foods {
        let pulse = (now * 4.0).sin() as f32;
        let glow_scale = 1.0 + 0.08 * pulse;
        let center = *food * cell + vec2(cell / 2.0, cell / 2.0);

        for i in (0..4).rev() {
            let r = cell * (0.55 + i as f32 * 0.22) * glow_scale;
            let a = 0.10 - i as f32 * 0.025;
            draw_circle(center.x, center.y, r, Color::new(FOOD_RED.r, FOOD_RED.g, FOOD_RED.b, a));
        }

        let radius = cell * 0.34 * glow_scale;
        draw_circle(center.x, center.y, radius, FOOD_RED);
        draw_circle(center.x - radius * 0.3, center.y - radius * 0.3, radius * 0.5, Color::new(1.0, 0.6, 0.6, 0.5));
        draw_circle(center.x + radius * 0.1, center.y - radius * 0.9, radius * 0.18, Color::new(0.35, 0.6, 0.2, 1.0));
        let leaf = center + vec2(-radius * 0.25, -radius);
        draw_circle(leaf.x, leaf.y, radius * 0.28, Color::new(0.35, 0.8, 0.3, 1.0));
        draw_rectangle(center.x - 1.0, center.y - radius * 1.15, 2.0, radius * 0.45, Color::new(0.5, 0.45, 0.2, 1.0));
    }
}

fn body_pos(snake: &[Vec2], prev: &[Vec2], i: usize, t: f32, wrap: bool, grid_w: usize, grid_h: usize) -> Vec2 {
    let prev_pos = if i < prev.len() { prev[i] } else { *prev.last().unwrap_or(&snake[i]) };
    let cur = snake[i];
    let mut delta = cur - prev_pos;
    if wrap {
        let gw = grid_w.max(1) as f32;
        let gh = grid_h.max(1) as f32;
        if delta.x > gw * 0.5 {
            delta.x -= gw;
        } else if delta.x < -gw * 0.5 {
            delta.x += gw;
        }
        if delta.y > gh * 0.5 {
            delta.y -= gh;
        } else if delta.y < -gh * 0.5 {
            delta.y += gh;
        }
        let pos = prev_pos + delta * t;
        return vec2(pos.x.rem_euclid(gw), pos.y.rem_euclid(gh));
    }
    prev_pos + delta * t
}

fn draw_trail(trail: &VecDeque<(Vec2, f64)>, cell: f32, now: f64) {
    for &(pos, t0) in trail {
        let age = now - t0;
        let k = (1.0 - age / 0.9).clamp(0.0, 1.0) as f32;
        if k <= 0.0 {
            continue;
        }
        let r = cell * 0.35 * k;
        draw_circle(
            pos.x * cell + cell / 2.0,
            pos.y * cell + cell / 2.0,
            r,
            Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.35 * k),
        );
    }
}

fn draw_snake(snake: &[Vec2], prev: &[Vec2], dir: Direction, t: f32, cell: f32, wrap: bool, grid_w: usize, grid_h: usize) {
    let len = snake.len();
    for i in (0..len).rev() {
        let pos = body_pos(snake, prev, i, t, wrap, grid_w, grid_h);

        let inset = cell * 0.08;
        let rect = Rect::new(
            pos.x * cell + inset,
            pos.y * cell + inset,
            cell - inset * 2.0,
            cell - inset * 2.0,
        );

        let body_t = i as f32 / len.max(1) as f32;
        let color = gradient_color(body_t);

        draw_rounded_rect(
            Rect::new(rect.x + 2.0, rect.y + 3.0, rect.w, rect.h),
            rect.w * 0.35,
            Color::new(0.0, 0.0, 0.0, 0.25),
        );
        draw_rounded_rect(rect, rect.w * 0.35, color);

        if i == 0 {
            draw_eyes(rect, dir, cell);
        }
    }
}

fn gradient_color(body_t: f32) -> Color {
    let hue = (120.0 - 360.0 * body_t).rem_euclid(360.0);
    let v = 0.95 - 0.30 * body_t;
    hsv_to_rgb(hue, 0.90, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::new(r + m, g + m, b + m, 1.0)
}

fn draw_eyes(head_rect: Rect, dir: Direction, cell: f32) {
    let d = dir.to_vec();
    let eye_r = cell * 0.06;
    let eye_offset = cell * 0.16;
    let perp = vec2(-d.y, d.x);

    let look = (cell * 0.22) * d;
    let e1 = vec2(head_rect.x + head_rect.w / 2.0, head_rect.y + head_rect.h / 2.0) + perp * eye_offset + d * 0.08 * cell;
    let e2 = vec2(head_rect.x + head_rect.w / 2.0, head_rect.y + head_rect.h / 2.0) - perp * eye_offset + d * 0.08 * cell;

    for e in [e1, e2] {
        draw_circle(e.x, e.y, eye_r * 1.7, Color::new(1.0, 1.0, 1.0, 0.9));
        draw_circle(e.x + look.x, e.y + look.y, eye_r, Color::new(0.1, 0.1, 0.1, 1.0));
    }
}

fn draw_eaten(eaten: &[EatenFruit], cell: f32, now: f64) {
    for e in eaten {
        let progress = ((now - e.start) / EAT_ANIM).min(1.0) as f32;
        let scale = 1.0 - 0.55 * progress;
        let alpha = if progress < 0.6 { 1.0 } else { 1.0 - (progress - 0.6) / 0.4 };

        let center = e.pos * cell + vec2(cell / 2.0, cell / 2.0);
        let r = cell * 0.34 * scale;

        draw_circle(center.x, center.y, r * 1.7, Color::new(FOOD_RED.r, FOOD_RED.g, FOOD_RED.b, alpha * 0.12));
        draw_circle(center.x, center.y, r, Color::new(FOOD_RED.r, FOOD_RED.g, FOOD_RED.b, alpha));
        draw_circle(center.x - r * 0.3, center.y - r * 0.3, r * 0.5, Color::new(1.0, 0.6, 0.6, alpha * 0.5));
        draw_circle(center.x + r * 0.1, center.y - r * 0.9, r * 0.16, Color::new(0.35, 0.6, 0.2, alpha));
    }
}

fn draw_centered(text: &str, x: f32, y: f32, size: f32, color: Color, shadow: bool) {
    let w = measure_text(text, None, size as u16, 1.0).width;
    let px = x - w / 2.0;
    if shadow {
        draw_text(text, px + 2.0, y + 2.0, size, Color::new(0.0, 0.0, 0.0, 0.5));
    }
    draw_text(text, px, y, size, color);
}

fn draw_hud(score: i32, speed_name: &str, autoplay: bool) {
    let margin = 14.0;

    let panel = Rect::new(margin, margin, 190.0, 40.0);
    draw_rounded_rect(panel, 10.0, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_circle(margin + 20.0, margin + 20.0, 8.0, FOOD_RED);
    draw_text(&format!("Score: {}  ({})", score, speed_name), margin + 34.0, margin + 27.0, 20.0, WHITE);

    if autoplay {
        draw_text("AUTO", screen_width() / 2.0 - 30.0, margin + 27.0, 20.0, ACCENT);
    }

    let info = format!("O: options  P: autoplay  |  +/- zoom  F fullscreen");
    let info_w = measure_text(&info, None, 15, 1.0).width;
    let info_panel = Rect::new(screen_width() - margin - info_w - 24.0, margin, info_w + 24.0, 30.0);
    draw_rounded_rect(info_panel, 9.0, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_text(&info, info_panel.x + 12.0, info_panel.y + 21.0, 15.0, Color::new(0.75, 0.8, 0.85, 1.0));
}

fn draw_start_screen(score: i32) {
    let width = screen_width();
    let height = screen_height();
    draw_rectangle(0.0, 0.0, width, height, Color::new(0.0, 0.0, 0.0, 0.35));

    draw_centered("SNAKE", width / 2.0, height * 0.3, 72.0, ACCENT, true);

    draw_centered("Eat the fruit, grow long", width / 2.0, height * 0.4, 22.0, Color::new(0.8, 0.85, 0.9, 1.0), true);

    let panel = Rect::new(width / 2.0 - 170.0, height * 0.5, 340.0, 170.0);
    draw_rounded_rect(panel, 14.0, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_text("Move: WASD or Arrows", panel.x + 30.0, panel.y + 38.0, 18.0, Color::new(0.85, 0.9, 0.95, 1.0));
    draw_text("Zoom: +/-   Fullscreen: F", panel.x + 30.0, panel.y + 70.0, 18.0, Color::new(0.85, 0.9, 0.95, 1.0));
    draw_text("Options: O   Autoplay: P", panel.x + 30.0, panel.y + 102.0, 18.0, Color::new(0.85, 0.9, 0.95, 1.0));
    draw_text("Restart game over: SPACE", panel.x + 30.0, panel.y + 134.0, 18.0, Color::new(0.85, 0.9, 0.95, 1.0));
    draw_centered("Press SPACE to start", width / 2.0, height * 0.78, 26.0, YELLOW, true);

    let _ = score;
}

fn draw_game_over(score: i32) {
    let width = screen_width();
    let height = screen_height();
    draw_rectangle(0.0, 0.0, width, height, Color::new(0.0, 0.0, 0.0, 0.45));

    let panel_w = 380.0;
    let panel_h = 200.0;
    let panel = Rect::new((width - panel_w) / 2.0, (height - panel_h) / 2.0 - 20.0, panel_w, panel_h);
    draw_rounded_rect(panel, 18.0, Color::new(0.0, 0.0, 0.0, 0.55));

    draw_centered("GAME OVER", width / 2.0, panel.y + 62.0, 44.0, FOOD_RED, true);
    draw_centered(&format!("Final Score: {}", score), width / 2.0, panel.y + 116.0, 30.0, WHITE, true);
    draw_centered("Press SPACE to restart", width / 2.0, panel.y + 172.0, 20.0, YELLOW, true);
}

fn draw_win_screen(score: i32) {
    let width = screen_width();
    let height = screen_height();
    draw_rectangle(0.0, 0.0, width, height, Color::new(0.0, 0.0, 0.0, 0.45));

    let panel_w = 420.0;
    let panel_h = 220.0;
    let panel = Rect::new((width - panel_w) / 2.0, (height - panel_h) / 2.0 - 20.0, panel_w, panel_h);
    draw_rounded_rect(panel, 18.0, Color::new(0.0, 0.0, 0.0, 0.55));

    draw_centered("YOU WIN!", width / 2.0, panel.y + 66.0, 48.0, ACCENT, true);
    draw_centered("The board is full", width / 2.0, panel.y + 118.0, 24.0, Color::new(0.8, 0.85, 0.9, 1.0), true);
    draw_centered(&format!("Final Score: {}", score), width / 2.0, panel.y + 158.0, 28.0, WHITE, true);
    draw_centered("Press SPACE to play again", width / 2.0, panel.y + 196.0, 20.0, YELLOW, true);
}

fn draw_options(options: &[GameOption], cursor: usize) {
    let width = screen_width();
    let height = screen_height();
    let (panel, rows, close) = options_layout(options.len());

    draw_rectangle(0.0, 0.0, width, height, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_rounded_rect(panel, 16.0, Color::new(0.06, 0.10, 0.14, 0.96));

    draw_centered("OPTIONS", width / 2.0, panel.y + 38.0, 30.0, ACCENT, true);

    for (i, opt) in options.iter().enumerate() {
        let r = rows[i];
        let selected = i == cursor;
        let bg = if selected { Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.16) } else { Color::new(1.0, 1.0, 1.0, 0.04) };
        draw_rounded_rect(r, 10.0, bg);
        if selected {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, ACCENT);
        }

        let name_color = if selected { WHITE } else { Color::new(0.8, 0.85, 0.9, 1.0) };
        draw_text(opt.name, r.x + 18.0, r.y + 28.0, 20.0, name_color);

        let value = opt.values[opt.idx];
        let value_color = if selected { ACCENT } else { Color::new(0.7, 0.75, 0.8, 1.0) };
        let vw = measure_text(value, None, 20, 1.0).width;
        let cx = r.x + r.w - 34.0;
        draw_text(value, cx - vw, r.y + 28.0, 20.0, value_color);
        if selected {
            draw_text("<", cx - vw - 24.0, r.y + 28.0, 20.0, ACCENT);
            draw_text(">", cx + 8.0, r.y + 28.0, 20.0, ACCENT);
        }
    }

    let hint = "Up/Down select   Left/Right change   O / Esc close";
    let hint_w = measure_text(hint, None, 16, 1.0).width;
    draw_text(hint, width / 2.0 - hint_w / 2.0, panel.y + panel.h - 18.0, 16.0, Color::new(0.75, 0.8, 0.85, 1.0));

    let mpos = Vec2::from(mouse_position());
    let hover = close.contains(mpos);
    draw_rounded_rect(close, 10.0, if hover { Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.25) } else { Color::new(1.0, 1.0, 1.0, 0.08) });
    draw_centered("Close", close.x + close.w / 2.0, close.y + 27.0, 20.0, if hover { WHITE } else { Color::new(0.8, 0.85, 0.9, 1.0) }, false);
}

fn build_hamilton_cycle(gw: usize, gh: usize) -> Option<Vec<i32>> {
    let gw = gw.max(2);
    let gh = gh.max(2);
    if gw % 2 == 0 {
        build_cycle_weven(gw, gh)
    } else if gh % 2 == 0 {
        let t = build_cycle_weven(gh, gw)?;
        let mut next = vec![-1i32; gw * gh];
        let idx = |x: usize, y: usize| y * gw + x;
        for (i, &n) in t.iter().enumerate() {
            if n < 0 {
                continue;
            }
            let (tx, ty) = (i % gh, i / gh);
            let (nx, ny) = (n as usize % gh, n as usize / gh);
            next[idx(ty, tx)] = idx(ny, nx) as i32;
        }
        Some(next)
    } else {
        None
    }
}

fn build_cycle_weven(gw: usize, gh: usize) -> Option<Vec<i32>> {
    let total = gw * gh;
    let mut next = vec![-1i32; total];
    let idx = |x: usize, y: usize| y * gw + x;
    let mut prev: Option<usize> = None;
    let link = |i: usize, next: &mut Vec<i32>, prev: &mut Option<usize>| {
        if let Some(p) = *prev {
            next[p] = i as i32;
        }
        *prev = Some(i);
    };
    for x in 0..gw {
        link(idx(x, 0), &mut next, &mut prev);
    }
    if gh >= 2 {
        link(idx(gw - 1, 1), &mut next, &mut prev);
        let mut c = gw - 1;
        loop {
            for y in 2..=gh - 1 {
                link(idx(c, y), &mut next, &mut prev);
            }
            link(idx(c - 1, gh - 1), &mut next, &mut prev);
            if c == 1 {
                for y in (1..gh - 1).rev() {
                    link(idx(0, y), &mut next, &mut prev);
                }
                next[idx(0, 1)] = idx(0, 0) as i32;
                break;
            }
            for y in (1..gh - 1).rev() {
                link(idx(c - 1, y), &mut next, &mut prev);
            }
            link(idx(c - 2, 1), &mut next, &mut prev);
            if c < 3 {
                break;
            }
            c -= 2;
        }
    }
    Some(next)
}

fn autoplay_dir(
    snake: &[Vec2],
    foods: &[Vec2],
    grid_w: usize,
    grid_h: usize,
    walls_solid: bool,
    self_collision: bool,
    current: Direction,
    looping: bool,
) -> Direction {
    let gw = grid_w.max(1);
    let gh = grid_h.max(1);
    if snake.is_empty() {
        return current;
    }
    let head = snake[0];
    let hx = head.x as i32;
    let hy = head.y as i32;
    if hx >= 0 && hy >= 0 && hx < gw as i32 && hy < gh as i32 {
        if let Some(cycle) = build_hamilton_cycle(gw, gh) {
            let h = hy as usize * gw + hx as usize;
            if h < cycle.len() {
                let nc = cycle[h];
                if nc >= 0 {
                    let nx = (nc as usize % gw) as i32;
                    let ny = (nc as usize / gw) as i32;
                    let step_dir = match (nx - hx, ny - hy) {
                        (0, -1) => Some(Direction::Up),
                        (0, 1) => Some(Direction::Down),
                        (-1, 0) => Some(Direction::Left),
                        (1, 0) => Some(Direction::Right),
                        _ => None,
                    };
                    if let Some(d) = step_dir {
                        let valid = (!walls_solid
                            || (nx >= 0 && ny >= 0 && nx < gw as i32 && ny < gh as i32))
                            && (!self_collision
                                || !snake
                                    .iter()
                                    .skip(1)
                                    .any(|s| s.x == nx as f32 && s.y == ny as f32));
                        if valid {
                            let short = snake.len() < (gw * gh) / 5;
                            if short {
                                return d;
                            }
                            let tail_reach = if self_collision {
                                let tail = snake[snake.len() - 1];
                                let mut blocked = vec![false; gw * gh];
                                for (i, s) in snake.iter().enumerate() {
                                    if i != snake.len() - 1 {
                                        blocked[s.y as usize * gw + s.x as usize] = true;
                                    }
                                }
                                bfs_survival(
                                    vec2(nx as f32, ny as f32),
                                    tail,
                                    foods,
                                    &blocked,
                                    gw,
                                    gh,
                                    walls_solid,
                                )
                                .0
                            } else {
                                true
                            };
                            if tail_reach {
                                return d;
                            }
                        }
                    }
                }
            }
        }
    }
    greedy_step(snake, foods, gw, gh, walls_solid, self_collision, current, looping)
}

fn greedy_step(
    snake: &[Vec2],
    foods: &[Vec2],
    gw: usize,
    gh: usize,
    walls_solid: bool,
    self_collision: bool,
    current: Direction,
    looping: bool,
) -> Direction {
    if snake.is_empty() {
        return current;
    }
    let head = snake[0];
    let hx = head.x as i32;
    let hy = head.y as i32;
    if hx < 0 || hy < 0 || hx >= gw as i32 || hy >= gh as i32 {
        return current;
    }
    let idx = |x: i32, y: i32| (y as usize) * gw + (x as usize);

    let dirs = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];

    let mut best: Option<((i32, i32, i32, i32), Direction)> = None;

    for dir in dirs {
        let step = dir.to_vec();
        let mut nx = hx + step.x as i32;
        let mut ny = hy + step.y as i32;
        if !walls_solid {
            nx = nx.rem_euclid(gw as i32);
            ny = ny.rem_euclid(gh as i32);
        } else if nx < 0 || ny < 0 || nx >= gw as i32 || ny >= gh as i32 {
            continue;
        }
        let ni = idx(nx, ny);
        if self_collision && snake.iter().skip(1).any(|s| s.x == nx as f32 && s.y == ny as f32) {
            continue;
        }

        let new_head = vec2(nx as f32, ny as f32);

        let tail_len = snake.len();
        let vacated_idx = idx(snake[tail_len - 1].x as i32, snake[tail_len - 1].y as i32);
        let tail_cell = snake[tail_len - 1];

        let mut blocked = vec![false; gw * gh];
        if self_collision {
            for seg in snake {
                let x = seg.x as i32;
                let y = seg.y as i32;
                let i = idx(x, y);
                if i != vacated_idx {
                    blocked[i] = true;
                }
            }
        }

        let (tail_reach, can_eat, dist_food, _tail_dist) =
            bfs_survival(new_head, tail_cell, foods, &blocked, gw, gh, walls_solid);

        let food_score = if can_eat {
            let mut s = 100_000;
            if dist_food < u32::MAX {
                s = 100_000 - dist_food as i32;
            }
            s
        } else {
            0
        };

        let flood = flood_count(ni, &blocked, gw, gh, walls_solid);

        let tie = if dir == current { 1 } else { 0 };
        let score = if looping {
            (tail_reach as i32, flood, food_score, tie)
        } else {
            (tail_reach as i32, food_score, flood, tie)
        };

        if best.as_ref().map_or(true, |b| score > b.0) {
            best = Some((score, dir));
        }
    }

    best.map(|b| b.1).unwrap_or(current)
}

fn bfs_survival(
    start: Vec2,
    tail: Vec2,
    foods: &[Vec2],
    blocked: &[bool],
    gw: usize,
    gh: usize,
    walls_solid: bool,
) -> (bool, bool, u32, u32) {
    let sx = start.x as i32;
    let sy = start.y as i32;
    let tx = tail.x as i32;
    let ty = tail.y as i32;
    let s = sy as usize * gw + sx as usize;
    if s >= blocked.len() || blocked[s] {
        return (false, false, u32::MAX, u32::MAX);
    }
    let is_adjacent = |x: i32, y: i32| {
        let dx = (x - tx).abs();
        let dy = (y - ty).abs();
        if walls_solid {
            dx + dy == 1
        } else {
            let wrapped = dx == 0 && dy == 1
                || dx == 1 && dy == 0
                || dx == gw as i32 - 1 && dy == 0
                || dx == 0 && dy == gh as i32 - 1;
            dx + dy == 1 || wrapped
        }
    };
    let dirs = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
    let mut dist = vec![u32::MAX; gw * gh];
    dist[s] = 0;
    let mut tail_dist = u32::MAX;
    let mut food_dist = u32::MAX;
    let mut queue = VecDeque::new();
    queue.push_back((sx, sy));
    while let Some((cx, cy)) = queue.pop_front() {
        let d = dist[cy as usize * gw + cx as usize];
        if is_adjacent(cx, cy) {
            tail_dist = tail_dist.min(d);
        }
        if foods.contains(&vec2(cx as f32, cy as f32)) {
            food_dist = food_dist.min(d);
        }
        for dir in dirs {
            let step = dir.to_vec();
            let mut nx = cx + step.x as i32;
            let mut ny = cy + step.y as i32;
            if !walls_solid {
                nx = nx.rem_euclid(gw as i32);
                ny = ny.rem_euclid(gh as i32);
            } else if nx < 0 || ny < 0 || nx >= gw as i32 || ny >= gh as i32 {
                continue;
            }
            let ni = ny as usize * gw + nx as usize;
            if dist[ni] != u32::MAX || blocked[ni] {
                continue;
            }
            dist[ni] = d + 1;
            queue.push_back((nx, ny));
        }
    }
    (tail_dist != u32::MAX, food_dist != u32::MAX, food_dist, tail_dist)
}

fn flood_count(
    start: usize,
    blocked: &[bool],
    gw: usize,
    gh: usize,
    walls_solid: bool,
) -> i32 {
    if blocked[start] {
        return 0;
    }
    let sx = (start % gw) as i32;
    let sy = (start / gw) as i32;
    let dirs = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
    let mut seen = vec![false; gw * gh];
    seen[start] = true;
    let mut count = 1;
    let mut queue = VecDeque::new();
    queue.push_back((sx, sy));
    while let Some((cx, cy)) = queue.pop_front() {
        for dir in dirs {
            let step = dir.to_vec();
            let mut nx = cx + step.x as i32;
            let mut ny = cy + step.y as i32;
            if !walls_solid {
                nx = nx.rem_euclid(gw as i32);
                ny = ny.rem_euclid(gh as i32);
            } else if nx < 0 || ny < 0 || nx >= gw as i32 || ny >= gh as i32 {
                continue;
            }
            let ni = ny as usize * gw + nx as usize;
            if seen[ni] || blocked[ni] {
                continue;
            }
            seen[ni] = true;
            count += 1;
            queue.push_back((nx, ny));
        }
    }
    count
}
