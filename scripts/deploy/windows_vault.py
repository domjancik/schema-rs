"""Windows Credential Manager storage scoped to Schéma deployment credentials."""
import ctypes
from ctypes import wintypes
CRED_TYPE_GENERIC=1
CRED_PERSIST_LOCAL_MACHINE=2
ERROR_NOT_FOUND=1168
class Credential(ctypes.Structure):
    _fields_=[('Flags',wintypes.DWORD),('Type',wintypes.DWORD),('TargetName',wintypes.LPWSTR),('Comment',wintypes.LPWSTR),('LastWritten',wintypes.FILETIME),('CredentialBlobSize',wintypes.DWORD),('CredentialBlob',ctypes.POINTER(ctypes.c_ubyte)),('Persist',wintypes.DWORD),('AttributeCount',wintypes.DWORD),('Attributes',ctypes.c_void_p),('TargetAlias',wintypes.LPWSTR),('UserName',wintypes.LPWSTR)]
api=ctypes.WinDLL('Advapi32.dll',use_last_error=True)
api.CredReadW.argtypes=[wintypes.LPCWSTR,wintypes.DWORD,wintypes.DWORD,ctypes.POINTER(ctypes.POINTER(Credential))];api.CredReadW.restype=wintypes.BOOL
api.CredWriteW.argtypes=[ctypes.POINTER(Credential),wintypes.DWORD];api.CredWriteW.restype=wintypes.BOOL
api.CredFree.argtypes=[ctypes.c_void_p];api.CredFree.restype=None
api.CredDeleteW.argtypes=[wintypes.LPCWSTR,wintypes.DWORD,wintypes.DWORD];api.CredDeleteW.restype=wintypes.BOOL
def check_target(target):
    if not target.startswith('schema-rs/') or len(target)>200:raise ValueError('Credential target must belong to schema-rs/')
def read(target):
    check_target(target);pointer=ctypes.POINTER(Credential)()
    if not api.CredReadW(target,CRED_TYPE_GENERIC,0,ctypes.byref(pointer)):
        if ctypes.get_last_error()==ERROR_NOT_FOUND:return None
        raise ctypes.WinError(ctypes.get_last_error())
    try:return ctypes.string_at(pointer.contents.CredentialBlob,pointer.contents.CredentialBlobSize).decode('utf-16-le')
    finally:api.CredFree(pointer)
def write(target,password,username='orangepi'):
    check_target(target);blob=password.encode('utf-16-le')
    if not blob or len(blob)>2560:raise ValueError('Invalid credential size')
    buffer=ctypes.create_string_buffer(blob)
    credential=Credential();credential.Type=CRED_TYPE_GENERIC;credential.TargetName=target;credential.Comment='Schéma Orange Pi sudo authentication';credential.UserName=username;credential.Persist=CRED_PERSIST_LOCAL_MACHINE;credential.CredentialBlobSize=len(blob);credential.CredentialBlob=ctypes.cast(buffer,ctypes.POINTER(ctypes.c_ubyte))
    try:
        if not api.CredWriteW(ctypes.byref(credential),0):raise ctypes.WinError(ctypes.get_last_error())
    finally:ctypes.memset(ctypes.addressof(buffer),0,ctypes.sizeof(buffer))
def delete(target):
    check_target(target)
    if not api.CredDeleteW(target,CRED_TYPE_GENERIC,0) and ctypes.get_last_error()!=ERROR_NOT_FOUND:raise ctypes.WinError(ctypes.get_last_error())
