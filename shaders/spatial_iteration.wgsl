// Heads and per-slot links replace cell prefix sums in ordinary playback.
// The contiguous reference remains available to paired diagnostic tests.
const LINKED_SPATIAL:bool=true;
fn spatial_first(cell:u32)->u32 {
 if(LINKED_SPATIAL){return offsets[cell];}
 if(cell==0u){return 0u;}return offsets[cell-1u];
}
fn spatial_end(cell:u32)->u32 {
 if(LINKED_SPATIAL){return INVALID;}return offsets[cell];
}
fn spatial_slot(cursor:u32)->u32 {
 if(LINKED_SPATIAL){return cursor;}return indices[cursor];
}
fn spatial_next(cursor:u32)->u32 {
 if(LINKED_SPATIAL){return indices[cursor];}return cursor+1u;
}
