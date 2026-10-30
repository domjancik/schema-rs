use crate::{engine::Command, model::*, server::Published};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    terminal::{self, ClearType},
};
use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::{Duration, Instant},
};
pub fn request(
    address: &str,
    method: &str,
    path: &str,
    body: Option<&serde_json::Value>,
    token: Option<&str>,
) -> Result<Vec<u8>> {
    if address.contains(['\r', '\n', '/']) || token.is_some_and(|s| s.contains(['\r', '\n'])) {
        return Err("Invalid server address or token".into());
    }
    let addr = address
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("No server address")?;
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let data = body.map(ToString::to_string).unwrap_or_default();
    let auth = token
        .map(|v| format!("X-Schema-Token: {v}\r\n"))
        .unwrap_or_default();
    write!(stream,"{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{auth}\r\n{data}",data.len()).map_err(|e|e.to_string())?;
    let mut response = Vec::new();
    stream
        .take(4 * 1024 * 1024)
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    let split = response
        .windows(4)
        .position(|v| v == b"\r\n\r\n")
        .ok_or("Invalid HTTP response")?;
    let headers = String::from_utf8_lossy(&response[..split]);
    let status = headers.split_whitespace().nth(1).unwrap_or("500");
    let mut payload = response[split + 4..].to_vec();
    if headers
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        let mut decoded = Vec::new();
        let mut remaining = payload.as_slice();
        loop {
            let end = remaining
                .windows(2)
                .position(|v| v == b"\r\n")
                .ok_or("Invalid HTTP chunk")?;
            let text = std::str::from_utf8(&remaining[..end]).map_err(|e| e.to_string())?;
            let length = usize::from_str_radix(text.split(';').next().unwrap_or(""), 16)
                .map_err(|e| e.to_string())?;
            if length == 0 {
                break;
            }
            remaining = &remaining[end + 2..];
            if remaining.len() < length + 2 {
                return Err("Truncated HTTP chunk".into());
            }
            decoded.extend_from_slice(&remaining[..length]);
            remaining = &remaining[length + 2..];
        }
        payload = decoded;
    }
    if status != "200" {
        return Err(String::from_utf8_lossy(&payload).into_owned());
    }
    Ok(payload)
}
fn widths(n: &Node) -> usize {
    n.children.iter().map(widths).sum::<usize>().max(1)
}
fn rows<'a>(n: &'a Node, x: usize, depth: usize, out: &mut Vec<(&'a Node, usize, usize, usize)>) {
    out.push((n, x, depth, widths(n)));
    let mut x = x;
    for child in &n.children {
        rows(child, x, depth + 1, out);
        x += widths(child);
    }
}
pub fn render_stack(root: &Node, selected: u64, zoom: u8, width: usize, height: usize) -> String {
    let (cell_w, cell_h) = match zoom.min(3) {
        0 => (12, 3),
        1 => (22, 5),
        2 => (30, 8),
        _ => (40, 11),
    };
    let mut nodes = Vec::new();
    rows(root, 0, 0, &mut nodes);
    let max = nodes.iter().map(|r| r.2).max().unwrap_or(0);
    let selected_row = nodes
        .iter()
        .find(|r| r.0.id == selected)
        .unwrap_or(&nodes[0]);
    let focus_x = selected_row.1 * cell_w;
    let focus_y = (max - selected_row.2) * cell_h;
    let offset_x = focus_x.saturating_sub(width / 3);
    let offset_y = focus_y.saturating_sub(height / 2);
    let mut canvas = vec![vec![b' '; width]; height];
    fn text(canvas: &mut [Vec<u8>], x: isize, y: isize, s: &str) {
        if y < 0 || y as usize >= canvas.len() {
            return;
        }
        for (i, b) in s.bytes().enumerate() {
            let x = x + i as isize;
            if x >= 0 && (x as usize) < canvas[y as usize].len() {
                canvas[y as usize][x as usize] = if b.is_ascii() { b } else { b'?' };
            }
        }
    }
    for (n, x, depth, span) in nodes {
        let x = (x * cell_w) as isize - offset_x as isize;
        let y = ((max - depth) * cell_h) as isize - offset_y as isize;
        let w = span * cell_w - 2;
        let horizontal = "-".repeat(w.saturating_sub(2));
        text(&mut canvas, x, y, &format!("+{horizontal}+"));
        for row in 1..cell_h - 1 {
            text(&mut canvas, x, y + row as isize, "|");
            text(&mut canvas, x + w as isize - 1, y + row as isize, "|");
        }
        text(
            &mut canvas,
            x,
            y + cell_h as isize - 1,
            &format!("+{horizontal}+"),
        );
        let name = if zoom == 0 {
            format!("{} #{}", if selected == n.id { ">" } else { " " }, n.id)
        } else {
            format!(
                "{} {}",
                if selected == n.id { ">" } else { " " },
                n.behavior
            )
        };
        text(
            &mut canvas,
            x + 2,
            y + 1,
            &name.chars().take(w.saturating_sub(4)).collect::<String>(),
        );
        if zoom >= 2 {
            for (i, (k, v)) in n.properties.iter().take(cell_h - 5).enumerate() {
                let line = format!(
                    "{k}: {v}{}",
                    if n.drivers.contains_key(k) {
                        " [~]"
                    } else {
                        ""
                    }
                );
                text(
                    &mut canvas,
                    x + 2,
                    y + 3 + i as isize,
                    &line.chars().take(w.saturating_sub(4)).collect::<String>(),
                );
            }
        }
        if zoom >= 3 {
            text(
                &mut canvas,
                x + 2,
                y + cell_h as isize - 3,
                "a input / w wrap / d driver",
            );
        }
    }
    canvas
        .into_iter()
        .map(|row| String::from_utf8(row).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}
struct Terminal;
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            terminal::LeaveAlternateScreen,
            cursor::Show
        );
    }
}
fn prompt(label: &str) -> Result<String> {
    terminal::disable_raw_mode().map_err(|e| e.to_string())?;
    let (_, height) = terminal::size().unwrap_or((100, 30));
    execute!(
        std::io::stdout(),
        cursor::MoveTo(0, height.saturating_sub(2)),
        terminal::Clear(ClearType::CurrentLine),
        cursor::Show
    )
    .map_err(|e| e.to_string())?;
    print!("{label}: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut value = String::new();
    let read = std::io::stdin()
        .read_line(&mut value)
        .map_err(|e| e.to_string());
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let _ = execute!(std::io::stdout(), cursor::Hide);
    read?;
    Ok(value.trim().to_owned())
}
fn send(address: &str, token: Option<&str>, revision: u64, command: Command) -> Result<()> {
    request(
        address,
        "POST",
        "/api/command",
        Some(&serde_json::json!({"expected_revision":revision,"command":command})),
        token,
    )?;
    Ok(())
}
pub fn run(address: &str, token: Option<&str>) -> Result<()> {
    let mut published: Published =
        serde_json::from_slice(&request(address, "GET", "/api/state", None, token)?)
            .map_err(|e| e.to_string())?;
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let _guard = Terminal;
    execute!(
        std::io::stdout(),
        terminal::EnterAlternateScreen,
        cursor::Hide
    )
    .map_err(|e| e.to_string())?;
    let (mut fixture_index, mut layer, mut node, mut zoom, mut scene, mut property) =
        (0usize, 0usize, 0u64, 2u8, false, 0usize);
    let mut marked: Option<(u64, bool)> = None;
    let mut message = "Connected. Tab switches scene / stack. ? for controls.".to_owned();
    let mut last = Instant::now() - Duration::from_secs(1);
    loop {
        if last.elapsed() > Duration::from_millis(100) {
            match request(address, "GET", "/api/state", None, token)
                .and_then(|v| serde_json::from_slice(&v).map_err(|e| e.to_string()))
            {
                Ok(p) => published = p,
                Err(e) => message = e,
            }
            last = Instant::now();
        }
        fixture_index = fixture_index.min(published.project.fixtures.len().saturating_sub(1));
        let fixture = published.project.fixtures.get(fixture_index);
        let root = fixture
            .and_then(|f| published.project.content.get(&f.id))
            .and_then(|v| v.get(layer))
            .and_then(Option::as_ref)
            .map(|s| &s.root);
        if root.is_some_and(|r| r.find(node).is_none()) {
            node = root.unwrap().id;
        }
        let (width, height) = terminal::size().unwrap_or((120, 40));
        let mut screen = Vec::new();
        screen.push(format!(
            "SCHEMA / RUST   {}   {}   frame {}   {:0.2}s   rev {}",
            if scene { "SCENE" } else { "STACK" },
            published.project.name,
            published.snapshot.frame,
            published.snapshot.time,
            published.snapshot.revision
        ));
        screen.push(format!(
            "Fixture {}/{}   Layer {}   Zoom {}   {}",
            fixture_index + 1,
            published.project.fixtures.len(),
            layer + 1,
            zoom,
            published.snapshot.output_status
        ));
        if let Some(f) = fixture {
            screen.push(format!(
                "{}  XYZ {:?}  dim {:.2}  {:?} u{}:{} groups {}",
                f.name,
                f.position,
                f.dimmer,
                f.model,
                f.universe,
                f.address,
                f.groups.join(",")
            ));
        } else {
            screen.push("No fixtures. Press n in scene mode to add one.".into())
        }
        if scene {
            let w = width as usize;
            let h = height.saturating_sub(9) as usize;
            let mut grid = vec![vec![b'.'; w]; h];
            for (i, f) in published.project.fixtures.iter().enumerate() {
                let x = ((f.position[0] + 10.) / 20. * w.saturating_sub(1) as f64)
                    .clamp(0., w.saturating_sub(1) as f64) as usize;
                let y = ((f.position[2] + 10.) / 20. * h.saturating_sub(1) as f64)
                    .clamp(0., h.saturating_sub(1) as f64) as usize;
                if h > 0 && w > 0 {
                    grid[y][x] = if i == fixture_index { b'@' } else { b'o' };
                }
            }
            screen.extend(grid.into_iter().map(|r| String::from_utf8(r).unwrap()));
        } else if let Some(root) = root {
            screen.extend(
                render_stack(
                    root,
                    node,
                    zoom,
                    width as usize,
                    height.saturating_sub(9) as usize,
                )
                .lines()
                .map(str::to_owned),
            );
        } else {
            screen
                .push("Empty layer. Press i to import ScenicScript or a to create a root.".into());
        }
        screen.push("Tab scene/stack | , . fixture | 1–9 / [ ] layer | +/- zoom | u undo r redo | space pause b blackout".into());
        screen.push(if scene{"Arrows move X/Z | y/Y height | [ ] dim | e fixture JSON | n add | Delete remove | s save"}else{"Arrows navigate | a input | w wrap | Delete block | D branch | x/X mark | v/V move | p property | d driver | i import | o export"}.into());
        screen.push(message.clone());
        let mut stdout = std::io::stdout();
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )
        .map_err(|e| e.to_string())?;
        for (i, line) in screen.iter().take(height as usize).enumerate() {
            queue!(stdout, cursor::MoveTo(0, i as u16)).map_err(|e| e.to_string())?;
            write!(
                stdout,
                "{}",
                line.chars()
                    .filter(char::is_ascii)
                    .take(width as usize)
                    .collect::<String>()
            )
            .map_err(|e| e.to_string())?;
        }
        stdout.flush().map_err(|e| e.to_string())?;
        if !event::poll(Duration::from_millis(30)).map_err(|e| e.to_string())? {
            continue;
        }
        let Event::Key(key) = event::read().map_err(|e| e.to_string())? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('q')
            || key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            break;
        }
        let revision = published.snapshot.revision;
        let mut command = None;
        let fixture = fixture.cloned();
        let mut result: Result<()> = Ok(());
        match key.code{
   KeyCode::Tab=>scene= !scene,KeyCode::Char(',')=>fixture_index=fixture_index.saturating_sub(1),KeyCode::Char('.')=>fixture_index=(fixture_index+1).min(published.project.fixtures.len().saturating_sub(1)),KeyCode::Char(c @ '1'..='9')=>layer=(c as usize-'1' as usize).min(published.project.fixtures.get(fixture_index).and_then(|f|published.project.content.get(&f.id)).map_or(3,Vec::len).saturating_sub(1)),KeyCode::Char('[')=>layer=layer.saturating_sub(1),KeyCode::Char(']')=>layer=(layer+1).min(published.project.fixtures.get(fixture_index).and_then(|f|published.project.content.get(&f.id)).map_or(3,Vec::len).saturating_sub(1)),
   KeyCode::Char('+')|KeyCode::Char('=')=>zoom=(zoom+1).min(3),KeyCode::Char('-')=>zoom=zoom.saturating_sub(1),
   KeyCode::Char('u')=>command=Some(Command::Undo),KeyCode::Char('r')=>command=Some(Command::Redo),KeyCode::Char(' ')=>command=Some(Command::Pause{paused:!published.snapshot.paused}),KeyCode::Char('b')=>command=Some(Command::Blackout{enabled:!published.snapshot.blackout}),
   KeyCode::Char('s')=>{result=request(address,"POST","/api/project/save",Some(&serde_json::json!({"name":"init"})),token).map(|_|());message="Saved layout, content, stack files and output configuration".into();},
   KeyCode::Char('?')=>message="Scene arrows: 0.25m. Stack: child above, parent below. p edits JSON literal; d attaches LFO. Undo excludes clock/blackout. q quits client.".into(),
   KeyCode::Char('n')if scene=>{let id=format!("fixture-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());let address=published.project.fixtures.iter().filter(|f|f.universe==0).map(|f|f.address+published.project.footprint(f).unwrap_or(0) as u16).max().unwrap_or(1);command=Some(Command::AddFixture{fixture:Fixture{marker_color:None,dmx_model:None,setup_channel:0,rotation:[0.;3],pixel_positions:vec![[0.;3]],pixel_reverse:false,id,name:"New fixture".into(),position:[0.;3],groups:vec![],model:FixtureModel::Rgb,universe:0,address,dimmer:1.}});},
   code=>if let Some(mut f)=fixture{let id=f.id.clone();if scene{match code{
    KeyCode::Left=>{f.position[0]-=0.25;command=Some(Command::MoveFixture{fixture:id,position:f.position})},KeyCode::Right=>{f.position[0]+=0.25;command=Some(Command::MoveFixture{fixture:id,position:f.position})},KeyCode::Up=>{f.position[2]-=0.25;command=Some(Command::MoveFixture{fixture:id,position:f.position})},KeyCode::Down=>{f.position[2]+=0.25;command=Some(Command::MoveFixture{fixture:id,position:f.position})},
    KeyCode::Char('y')|KeyCode::Char('Y')=>{f.position[1]+=if code==KeyCode::Char('y'){0.25}else{-0.25};command=Some(Command::MoveFixture{fixture:id,position:f.position})},
    KeyCode::Char('[')|KeyCode::Char(']')=>command=Some(Command::SetDimmer{fixture:id,value:(f.dimmer+if code==KeyCode::Char(']'){0.05}else{-0.05}).clamp(0.,1.)}),
    KeyCode::Delete=>{if prompt("Type DELETE to remove fixture")?=="DELETE"{command=Some(Command::RemoveFixture{fixture:id})}},
    KeyCode::Char('e')=>{let current=serde_json::to_string(&f).unwrap();match prompt(&format!("Fixture JSON (current {current})")).and_then(|s|serde_json::from_str(&s).map_err(|e|e.to_string())){Ok(fixture)=>command=Some(Command::SetFixture{fixture}),Err(e)=>result=Err(e)}},_=>{}
   }}else{match code{
    KeyCode::Char('a')=>{let behavior=prompt("Behavior key")?;if let Some(root)=root{command=Some(Command::AddInput{index:None,fixture:id,layer,node:if root.find(node).is_some(){node}else{root.id},behavior})}else{result=crate::scenic::new_node(&behavior,1).map(|root|command=Some(Command::ReplaceLayer{fixture:id,layer,stack:Some(Stack{enabled:true,name:"New stack".into(),root})}));}},
    KeyCode::Char('x') | KeyCode::Char('X') => {marked=Some((node,code==KeyCode::Char('X')));message="Move marked: navigate to target, v append / V insert below".into();},
    KeyCode::Char('v') | KeyCode::Char('V') => {if let Some((source,branch))=marked.take(){command=Some(Command::MoveNode{fixture:id,layer,node:source,target:node,branch,below:code==KeyCode::Char('V')});}},
    KeyCode::Char('w')=>command=Some(Command::WrapNode{fixture:id,layer,node,behavior:prompt("Wrapper behavior key")?}),KeyCode::Delete=>command=Some(Command::DeleteNode{fixture:id,layer,node,branch:false}),KeyCode::Char('D')=>{if prompt("Type DELETE to remove branch")?=="DELETE"{command=Some(Command::DeleteNode{fixture:id,layer,node,branch:true})}},
    KeyCode::Char('p')=>{if let Some(n)=root.and_then(|r|r.find(node)){let keys:Vec<_>=n.properties.keys().cloned().collect();if !keys.is_empty(){let key=prompt(&format!("Property key ({})",keys.join(", ")))?;let text=prompt("JSON literal")?;result=serde_json::from_str(&text).map_err(|e|e.to_string()).map(|value|command=Some(Command::SetProperty{fixture:id,layer,node,key,value}));property=property.saturating_add(1);}}},
    KeyCode::Char('d')=>{let key=prompt("Numeric property to drive")?;command=Some(Command::SetDriver{fixture:id,layer,node,key,driver:Some(crate::scenic::new_node("Create.Value.LFO",1)?)})},
    KeyCode::Char('i')=>{let path=prompt("ScenicScript file")?;result=std::fs::read_to_string(path).map_err(|e|e.to_string()).and_then(|yaml|request(address,"POST","/api/import",Some(&serde_json::json!({"fixture":id,"layer":layer,"yaml":yaml,"expected_revision":revision})),token).map(|_|()));},
    KeyCode::Char('o')=>{let path=prompt("Export ScenicScript path")?;result=request(address,"GET",&format!("/api/export/{id}/{layer}"),None,token).and_then(|bytes|std::fs::write(path,bytes).map_err(|e|e.to_string()));},
    KeyCode::Up|KeyCode::Down|KeyCode::Left|KeyCode::Right=>{if let Some(root)=root{let mut order=Vec::new();rows(root,0,0,&mut order);if code==KeyCode::Up{if let Some(child)=root.find(node).and_then(|n|n.children.first()){node=child.id;}}else{fn parent(n:&Node,id:u64)->Option<&Node>{if n.children.iter().any(|c|c.id==id){Some(n)}else{n.children.iter().find_map(|c|parent(c,id))}}if let Some(p)=parent(root,node){if code==KeyCode::Down{node=p.id}else{let i=p.children.iter().position(|c|c.id==node).unwrap();node=p.children[if code==KeyCode::Left{i.saturating_sub(1)}else{(i+1).min(p.children.len()-1)}].id;}}}}},_=>{}
   }}},
  }
        if let Some(command) = command {
            result = send(address, token, revision, command);
            if result.is_ok() {
                message = "Edit accepted by engine".into();
            }
        }
        if let Err(error) = result {
            message = error;
        }
        last = Instant::now() - Duration::from_secs(1);
    }
    Ok(())
}
