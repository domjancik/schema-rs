"""User-operated masked Vercel token entry; secrets never enter logs or arguments."""
import argparse,json,pathlib,re,time,tkinter as tk,urllib.request
from tkinter import simpledialog,messagebox
from core import atomic,Database
from delivery import PUBLICATION_CREDENTIAL
from device import windows_vault

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--state',type=pathlib.Path,required=True);args=parser.parse_args();root=args.state.resolve();config=json.loads((root/'config.json').read_bytes());publication=config['publication']
 if not re.fullmatch(r'prj_[A-Za-z0-9]+',publication['project_id'])or not re.fullmatch(r'team_[A-Za-z0-9]+',publication['org_id']):raise ValueError('Invalid enrolled project')
 ui=tk.Tk();ui.withdraw()
 token=simpledialog.askstring('Schéma public publishing','Enter a Vercel deployment token for the Schéma project. It will be saved in Windows Credential Manager.',show='*',parent=ui)
 if not token:ui.destroy();return
 try:
  if len(token)>1200 or any(character.isspace()for character in token):raise ValueError('Invalid token format')
  url='https://api.vercel.com/v9/projects/'+publication['project_id']+'?teamId='+publication['org_id']
  with urllib.request.urlopen(urllib.request.Request(url,headers={'Authorization':'Bearer '+token}),timeout=15)as response:project=json.load(response)
  if project['id']!=publication['project_id']:raise ValueError('Unexpected project')
  windows_vault.write(PUBLICATION_CREDENTIAL,token,username='Schéma public publishing');token=None
  publication['token_credential']=PUBLICATION_CREDENTIAL;atomic(root/'config.json',(json.dumps(config,indent=2)+'\n').encode());db=Database(root/'updates.sqlite3');db.set('delivery_retry_at',time.time());db.close()
  messagebox.showinfo('Schéma','Credential saved securely. The update server will retry public publication.',parent=ui)
 except Exception:
  messagebox.showerror('Schéma','Token setup did not complete. Check access to the enrolled project and try again.',parent=ui)
 finally:token=None;ui.destroy()
if __name__=='__main__':main()
