use std::ffi::CStr;

use glib::translate::{FromGlibPtrFull, ToGlibPtr, ToGlibPtrMut};
use poppler::{Document, IndexIter, ffi};

/// One heading of the document outline, pointing at a zero-based page.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineEntry {
    pub title: String,
    pub page: usize,
    pub depth: usize,
}

pub fn outline(document: &Document) -> Vec<OutlineEntry> {
    // SAFETY: a null iterator means the document has no outline; a non-null one is owned here.
    let raw = unsafe { ffi::poppler_index_iter_new(document.to_glib_none().0) };
    if raw.is_null() {
        return Vec::new();
    }
    let mut iter = unsafe { IndexIter::from_glib_full(raw) };
    let mut entries = Vec::new();
    walk(document, &mut iter, 0, &mut entries);
    entries
}

fn walk(document: &Document, iter: &mut IndexIter, depth: usize, entries: &mut Vec<OutlineEntry>) {
    loop {
        if let Some((title, page)) = heading(document, iter) {
            entries.push(OutlineEntry { title, page, depth });
        }
        if let Some(mut child) = iter.child() {
            walk(document, &mut child, depth + 1, entries);
        }
        if !iter.next() {
            break;
        }
    }
}

fn heading(document: &Document, iter: &mut IndexIter) -> Option<(String, usize)> {
    // SAFETY: the action is owned by us until `poppler_action_free`; its union is read only
    // through the variant its `type_` names, and every pointer is null-checked.
    unsafe {
        let action = ffi::poppler_index_iter_get_action(iter.to_glib_none_mut().0);
        if action.is_null() {
            return None;
        }
        let heading = if (*action).type_ == ffi::POPPLER_ACTION_GOTO_DEST {
            let goto = (*action).goto_dest;
            let title = (!goto.title.is_null())
                .then(|| CStr::from_ptr(goto.title).to_string_lossy().into_owned());
            title.zip(destination_page(document, goto.dest))
        } else {
            None
        };
        ffi::poppler_action_free(action);
        heading
    }
}

/// # Safety
/// `dest` must be null or point at a live `PopplerDest`.
unsafe fn destination_page(document: &Document, dest: *mut ffi::PopplerDest) -> Option<usize> {
    if dest.is_null() {
        return None;
    }
    let page_number = unsafe {
        if (*dest).type_ == ffi::POPPLER_DEST_NAMED && !(*dest).named_dest.is_null() {
            let name = CStr::from_ptr((*dest).named_dest).to_string_lossy();
            let named = document.find_dest(&name)?;
            let raw: *const ffi::PopplerDest = named.to_glib_none().0;
            (*raw).page_num
        } else {
            (*dest).page_num
        }
    };
    usize::try_from(page_number).ok()?.checked_sub(1)
}
