//! Level 5's routes (#52): from where BLOB is, over the whole map as the map
//! reads the rooms, to the nearest missing core pieces, and to the core
//! while a piece it needs is carried. As ZX Sidekick has them
//! (zx-sidekick/zx-sidekick#48, #52, #54).

use starquake::map::{Graph, Place, Step};
use starquake::pickups::{Item, RoomSet};

/// How many of the nearest missing pieces the piece route can switch
/// between.
pub const NEAREST: usize = 3;

/// The routes from `start`: to the missing `pieces`, the nearest
/// [`NEAREST`] rooms of them nearest first, each to the part of its room a
/// piece lies in once it has been put there (row 0 until then: its room);
/// and to `core_room` while `carrying` a piece the core needs, `None`
/// otherwise or when there is no way. A teleporter between two booths in
/// `booths` counts as one step.
pub fn routes(
    graph: &Graph,
    start: Place,
    booths: &[u16],
    pieces: &[Item],
    carrying: bool,
    core_room: u16,
) -> (Vec<Vec<Step>>, Option<Vec<Step>>) {
    let mut found: Vec<(usize, u16, Vec<Step>)> = Vec::new();
    for item in pieces {
        let mut rooms = RoomSet::default();
        let mut places = Vec::new();
        let spot = (item.row() != 0)
            .then(|| {
                (
                    item.room(),
                    graph.part_at(item.room(), item.row(), item.col()),
                )
            })
            .filter(|&(_, part)| part != 0);
        match spot {
            Some(place) => places.push(place),
            None => rooms.set(item.room(), true),
        }
        if let Some(route) = graph.route(start, booths, &rooms, &places) {
            found.push((route.len(), item.room(), route));
        }
    }
    found.sort_by_key(|(steps, room, _)| (*steps, *room));
    let mut piece: Vec<Vec<Step>> = Vec::new();
    let mut rooms: Vec<u16> = Vec::new();
    for (_, room, route) in found {
        if piece.len() == NEAREST {
            break;
        }
        if !rooms.contains(&room) {
            rooms.push(room);
            piece.push(route);
        }
    }
    let core = carrying
        .then(|| {
            let mut room = RoomSet::default();
            room.set(core_room, true);
            graph.route(start, booths, &room, &[])
        })
        .flatten();
    (piece, core)
}

/// Which of the nearest pieces' rooms `ends` the piece route leads to, and
/// the room to remember as chosen. The room `chosen` stays chosen while it
/// is among them; gone, the route goes back to the nearest. A `switch`
/// moves to the next, and from the last back to the nearest, which is no
/// choice: the route then follows whichever piece is nearest.
pub fn choose(ends: &[u16], chosen: Option<u16>, switch: bool) -> (usize, Option<u16>) {
    let found = chosen.and_then(|c| ends.iter().position(|&e| e == c));
    if switch && ends.len() > 1 {
        let next = (found.unwrap_or(0) + 1) % ends.len();
        (next, (next != 0).then(|| ends[next]))
    } else {
        (found.unwrap_or(0), found.and(chosen))
    }
}

/// The steps a route walks, from `here`, as pairs of rooms; teleporter
/// jumps are left out.
pub fn walked_steps(here: u16, route: Option<&[Step]>) -> Vec<(u16, u16)> {
    let mut steps = Vec::new();
    let mut from = here;
    for step in route.unwrap_or_default() {
        if !step.teleport {
            steps.push((from, step.room));
        }
        from = step.room;
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_goes_through_the_three_and_back_to_the_nearest() {
        let ends = [101, 102, 103];
        assert_eq!(choose(&ends, None, false), (0, None), "the nearest");
        assert_eq!(choose(&ends, None, true), (1, Some(102)));
        assert_eq!(choose(&ends, Some(102), false), (1, Some(102)), "kept");
        assert_eq!(choose(&ends, Some(102), true), (2, Some(103)));
        assert_eq!(choose(&ends, Some(103), true), (0, None), "round again");
        // The chosen room moved nearer: it stays chosen.
        assert_eq!(choose(&[102, 101, 103], Some(102), false), (0, Some(102)));
        // The chosen room is gone: back to the nearest.
        assert_eq!(choose(&[101, 103, 104], Some(102), false), (0, None));
        assert_eq!(choose(&[101, 103, 104], Some(102), true), (1, Some(103)));
        assert_eq!(
            choose(&[101], None, true),
            (0, None),
            "one: nothing to switch to"
        );
        assert_eq!(choose(&[], Some(102), true), (0, None), "none");
    }

    #[test]
    fn walked_steps_leave_out_the_jumps() {
        let route = [
            Step {
                room: 11,
                teleport: false,
            },
            Step {
                room: 300,
                teleport: true,
            },
            Step {
                room: 316,
                teleport: false,
            },
        ];
        assert_eq!(walked_steps(10, Some(&route)), [(10, 11), (300, 316)]);
        assert!(walked_steps(10, None).is_empty());
    }
}
