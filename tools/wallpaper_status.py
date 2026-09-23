"""Read current v46 wallpaper title from Windows desktop child windows."""
import ctypes

u = ctypes.windll.user32
callback = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)
found = []


def visit(hwnd, _):
    title = ctypes.create_unicode_buffer(512)
    u.GetWindowTextW(hwnd, title, 512)
    if title.value.startswith("Primitive World v46"):
        found.append((hex(hwnd), title.value))
    u.EnumChildWindows(hwnd, callback(child), None)
    return True


def child(hwnd, _):
    title = ctypes.create_unicode_buffer(512)
    u.GetWindowTextW(hwnd, title, 512)
    if title.value.startswith("Primitive World v46"):
        found.append((hex(hwnd), title.value))
    return True


u.EnumWindows(callback(visit), None)
for handle, title in found:
    print(handle, title)
