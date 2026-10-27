# Implementation developments for scene lab

## Baseline: 9427a13

- Studio and schémaboy now have a local fullscreen toggle. Native browser enter/exit and phone-width touch were verified. Unsupported browsers hide the control; Escape synchronizes its state.
- Add buttons occupy the top interaction plane above adjacent blocks and fold controls (`cd74310`).
- Both changes are active in the Windows frontend at port 8788 and the Orange Pi embedded UI. Pi activation preserved authored scene and undo/redo; OLA was not restarted.
- Named project storage, native catalog, cyclic daily dimming, custom DMX models, universe patching and durable history are described in the main docs. Read compatibility limits before designing examples.

No implementation changes are requested from the scene lab. Report bugs and additions with stable SL IDs and reproduction evidence.
