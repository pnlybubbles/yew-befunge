#![recursion_limit="128"]
#[macro_use]
extern crate yew;

use std::time::Duration;
use std::char;
use yew::html::*;
use yew::services::timeout::TimeoutService;

fn main() {
  let model = init_model();
  program(model, update, view);
}

struct Model {
  input: String,
  interval: String,
  time: u64,
  befunge: Befunge,
}

struct Befunge {
  source: Array2d<char>,
  cursor: (i64, i64),
  direction: Direction,
  running: bool,
  mode: Mode,
  stack: Stack,
  output: String
}

type Stack = Vec<i64>;
#[derive(Debug)]
enum Mode { StringMode, End, None }
#[derive(Debug)]
enum Direction { Up, Down, Left, Right }
type Array2d<T> = Vec<Vec<T>>;

const DEFAULT_INTERVAL: f64 = 200.0;

const DEFAULT_INPUT: &str = "2>:1->1-00p::00g:       v
         v%-g00::_v#!`\\-_$$.v
     ^g00_                  v
 ^+1                        <
                  >       :.^";

fn init_model() -> Model {
  Model {
    input: DEFAULT_INPUT.to_string(),
    interval: format!("{}", DEFAULT_INTERVAL),
    time: 0,
    befunge: Befunge {
      source: string_to_array(DEFAULT_INPUT),
      cursor: (0, 0),
      direction: Direction::Right,
      running: false,
      mode: Mode::End,
      stack: vec![],
      output: "".to_string(),
    }
  }
}

enum Msg {
  Input(String),
  Interval(String),
  Toggle,
  Step,
  Reset,
  Tick,
}

fn update(context: &mut Context<Msg>, model: &mut Model, msg: Msg) {
  match msg {
    Msg::Input(input) => {
      // model.befunge.source = string_to_array(input.as_str());
      model.input = input;
    },
    Msg::Interval(interval) => {
      model.interval = interval;
    },
    Msg::Toggle => {
      match model.befunge.mode {
        Mode::End => model.befunge = init_befunge(model),
        _ => model.befunge.running = !model.befunge.running,
      }
      if model.befunge.running {
        context.timeout(Duration::from_millis(0), || Msg::Tick);
      }
      match model.befunge.mode {
        Mode::End => model.time = 0,
        _ => (),
      }
    },
    Msg::Reset => {
      model.befunge = Befunge {
        cursor: (0, 0),
        direction: Direction::Right,
        stack: vec![],
        output: "".to_string(),
        running: false,
        source: string_to_array(model.input.as_str()),
        mode: Mode::End,
      };
      model.time = 0;
    },
    Msg::Tick => {
      if model.befunge.running {
        let frame = (1.0 / model.interval
          .parse()
          .unwrap_or(DEFAULT_INTERVAL)
          .max(0.0001).min(1.0))
          .round() as usize;
        for _ in 0..frame {
          process(&mut model.befunge)
        }
        model.time += frame as u64;
        let ms = model.interval
          .parse()
          .unwrap_or(DEFAULT_INTERVAL as u64)
          .max(0).min(5000);
        context.timeout(Duration::from_millis(ms), || Msg::Tick);
      }
    },
    Msg::Step => {
      match model.befunge.mode {
        Mode::End => model.befunge = init_befunge(model),
        _ => (),
      }
      model.befunge.running = false;
      model.time += 1;
      process(&mut model.befunge);
    },
  }
}

fn init_befunge(model: &Model) -> Befunge {
  Befunge {
    cursor: (-1, 0),
    direction: Direction::Right,
    stack: vec![],
    output: "".to_string(),
    running: true,
    source: string_to_array(model.input.as_str()),
    mode: Mode::None,
  }
}

fn string_to_array(source: &str) -> Array2d<char> {
  source.split("\n").map( |v|
    v.chars().collect()
  ).collect()
}

fn cyclic_index<T>(a: &Vec<T>, i: i64) -> Option<i64> {
  let l = a.len() as i64;
  if l == 0 { None } else { Some(i % l) }
}

fn cyclic_index2d<T>(a: &Array2d<T>, cursor: (i64, i64)) -> Option<(i64, i64)> {
  let (x, y) = cursor;
  let cy = cyclic_index(&a, y);
  let cx = cy
    .and_then( |cy_| a.get(cy_ as usize) )
    .and_then( |row| cyclic_index(row, x) );
  cx.and_then( |cx_| cy.map( |cy_| (cx_, cy_) ) )
}

fn get2d<T: Clone>(a: &Array2d<T>, cursor: (i64, i64)) -> Option<T> {
  let (x, y) = cursor;
  a.get(y as usize)
    .and_then( |row| row.get(x as usize) )
    .cloned()
}

fn set2d<T>(cursor: (i64, i64), v: T, a: &mut Array2d<T>) {
  let (x, y) = cursor;
  a[y as usize][x as usize] = v;
}

// fn indexed_map2d<T, S, F: Fn((i64, i64), &T) -> S>(f: F, a: &Array2d<T>) -> Array2d<S> {
//   a.iter().enumerate().map( |(y, row)|
//     row.iter().enumerate().map( |(x, c)| f((x as i64, y as i64), c)).collect()
//   ).collect()
// }

fn walk_next<T>(a: &Array2d<T>, direction: &Direction, cursor: (i64, i64)) -> (i64, i64) {
  let (x, y) = cursor;
  let cursor_candidate = match *direction {
    Direction::Left  => (x - 1, y),
    Direction::Right => (x + 1, y),
    Direction::Up    => (x, y - 1),
    Direction::Down  => (x, y + 1),
  };
  cyclic_index2d(&a, cursor_candidate).unwrap_or((0, 0))
}

fn process(b: &mut Befunge) {
  let cursor = walk_next(&b.source, &b.direction, b.cursor);
  let cell = get2d(&b.source, cursor).unwrap_or(' ');
  match b.mode {
    Mode::End => (),
    Mode::StringMode => {
      b.cursor = cursor;
      if cell != '"' {
        b.stack.push(cell as i64);
      } else {
        commands(cell, cursor, b);
      }
    },
    Mode::None => {
      b.cursor = cursor;
      commands(cell, cursor, b);
    }
  }
}

fn calc<F: Fn(i64, i64) -> i64>(s: &mut Stack, f: F) {
  let y = s.pop().unwrap_or(0);
  let x = s.pop().unwrap_or(0);
  s.push(f(x, y));
}

fn commands(cell: char, cursor: (i64, i64), b: &mut Befunge) {
  match cell {
    '<' => b.direction = Direction::Left,
    '>' => b.direction = Direction::Right,
    '^' => b.direction = Direction::Up,
    'v' => b.direction = Direction::Down,
    ' ' => (),
    '_' => {
      let v = b.stack.pop().unwrap_or(0);
      b.direction = if v == 0 { Direction::Right } else { Direction::Left };
    },
    '|' => {
      let v = b.stack.pop().unwrap_or(0);
      b.direction = if v == 0 { Direction::Down } else { Direction::Up };
    },
    '#' => b.cursor = walk_next(&b.source, &b.direction, cursor),
    '@' => {
      b.running = false;
      b.mode = Mode::End;
    },
    '0' => b.stack.push(0),
    '1' => b.stack.push(1),
    '2' => b.stack.push(2),
    '3' => b.stack.push(3),
    '4' => b.stack.push(4),
    '5' => b.stack.push(5),
    '6' => b.stack.push(6),
    '7' => b.stack.push(7),
    '8' => b.stack.push(8),
    '9' => b.stack.push(9),
    '"' => b.mode = match b.mode {
      Mode::StringMode => Mode::None,
      _ => Mode::StringMode,
    },
    '.' => {
      let v = b.stack.pop().unwrap_or(0);
      b.output = format!("{}{} ", b.output, v);
    },
    ',' => {
      let v = b.stack.pop().unwrap_or(0);
      b.output = format!("{}{}", b.output,
        char::from_u32(v as u32).unwrap_or(' ')
      );
    },
    '+' => calc( &mut b.stack, |x, y| x + y ),
    '-' => calc( &mut b.stack, |x, y| x - y ),
    '*' => calc( &mut b.stack, |x, y| x * y ),
    '/' => calc( &mut b.stack, |x, y| x / y ),
    '%' => calc( &mut b.stack, |x, y| x % y ),
    '`' => calc( &mut b.stack, |x, y| if x > y { 1 } else { 0 } ),
    '!' => {
      let v = b.stack.pop().unwrap_or(0);
      b.stack.push(if v == 0 { 1 } else { 0 });
    },
    ':' => {
      let v = b.stack.pop().unwrap_or(0);
      b.stack.push(v);
      b.stack.push(v);
    },
    '\\' => {
      let y = b.stack.pop().unwrap_or(0);
      let x = b.stack.pop().unwrap_or(0);
      b.stack.push(y);
      b.stack.push(x);
    },
    '$' => {
      b.stack.pop();
    },
    'g' => {
      let y = b.stack.pop().unwrap_or(0);
      let x = b.stack.pop().unwrap_or(0);
      let c = get2d(&b.source, (x, y))
        .map( |v| v as i64 )
        .unwrap_or(0);
      b.stack.push(c);
    },
    'p' => {
      let y = b.stack.pop().unwrap_or(0);
      let x = b.stack.pop().unwrap_or(0);
      let v = b.stack.pop().unwrap_or(0);
      set2d((x, y), char::from_u32(v as u32).unwrap_or(' '), &mut b.source);
    },
    _ => (),
  }
}

fn view(model: &Model) -> Html<Msg> {
  html! {
    <div class="main", >
      <h1>{ "Befunge" }</h1>
      <div>
        <textarea
          class="text",
          type="text",
          oninput=|e: InputData| Msg::Input(e.value),
          value=&model.input,
          placeholder="This textarea will not work! Sorry :(",
          rows=10,
          cols=80, />
      </div>
      <input
        class="text",
        type="text",
        oninput=|e: InputData| Msg::Interval(e.value),
        value=&model.interval, />
      <input
        class="button",
        type="button",
        onclick=|_| Msg::Toggle,
        value=&if model.befunge.running { "stop" } else { "run" }, />
      <input class="button", type="button", onclick=|_| Msg::Step, value=&"step", />
      <input class="button", type="button", onclick=|_| Msg::Reset, value=&"reset", />
      <div>
        <div class="text", >
          { colorize(&model.befunge.source, model.befunge.cursor) }
        </div>
      </div>
      <div>
        <div class="text", >
          { model.befunge.stack.iter().map( |v| format!("{}", v) ).collect::<Vec<_>>().join(" ") }
        </div>
      </div>
      <div>
        <pre class="text", >
          { &model.befunge.output }
        </pre>
      </div>
      <div>{ format!("{}", model.time) }</div>
      <div>
        <a
          class="footer",
          href="https://github.com/pnlybubbles/yew-befunge",
          target="_blank", >
          { "source "}
        </a>
      </div>
    </div>
  }
}

fn fix_char_width(x: char) -> char {
  let ac = x as u32;
  if 33 <= ac && ac <= 126 {
    x
  } else {
    char::from_u32(160).unwrap_or(' ')
  }
}

fn colorize(source: &Array2d<char>, cursor: (i64, i64)) -> Html<Msg> {
  let (cx, cy) = cursor;
  html! {
    <div>
      {
        for source.iter().enumerate().map( |(y, row)| {
          html! {
            <div>
              {
                for row.iter().enumerate().map( |(x, cell)| {
                  html! {
                    <span class=if x as i64 == cx && y as i64 == cy { "cursor" } else { "" }, >
                      { fix_char_width(*cell).to_string() }
                    </span>
                  }
                })
              }
            </div>
          }
        })
      }  
    </div>
  }
}
