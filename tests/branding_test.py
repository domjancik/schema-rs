"""Visible product names carry the accent; protocol/file identifiers remain stable."""
import pathlib,re,unittest,json
from html.parser import HTMLParser
ROOT=pathlib.Path(__file__).resolve().parents[1]
class VisibleNames(HTMLParser):
 def __init__(self):super().__init__();self.hidden=0;self.text=[]
 def handle_starttag(self,tag,attrs):
  if tag in ['script','style']:self.hidden+=1
  for key,value in attrs:
   if key in ['title','alt','aria-label','placeholder'] and value:self.text.append(value)
 def handle_endtag(self,tag):
  if tag in ['script','style']:self.hidden=max(0,self.hidden-1)
 def handle_data(self,text):
  if not self.hidden:self.text.append(text)
class Branding(unittest.TestCase):
 def test_visible_html_product_names_and_install_manifest_use_accents(self):
  for path in list((ROOT/'web').glob('*.html'))+list((ROOT/'scripts/publication').glob('*.html'))+list((ROOT/'scripts/update_server/ui').glob('*.html')):
   visible=VisibleNames();visible.feed(path.read_text(encoding='utf-8'))
   for text in visible.text:self.assertIsNone(re.search(r'(?i)\bschema[- ·]s\b|\bschemaboy\b|\bschema(?![-._a-z0-9])',text),(str(path),text))
  manifest=json.loads((ROOT/'web/app.webmanifest').read_text(encoding='utf-8'));self.assertIn('Schéma',manifest['name']);self.assertEqual(manifest['short_name'],'schémaboy')
 def test_terminal_and_desktop_menu_labels_use_accents(self):
  self.assertTrue('SCHÉMA / RUST' in (ROOT/'src/tui.rs').read_text(encoding='utf-8'),'Terminal product heading must be accented')
  swift=(ROOT/'scripts/macos/Launcher.swift').read_text(encoding='utf-8')
  for text in ['Quit Schéma-s','Start Schéma-s at login','Schéma-s could not run']:self.assertTrue(text in swift,text)
 def test_linux_service_display_name_uses_accent(self):
  self.assertTrue('Description=Schéma-s lighting engine' in (ROOT/'scripts/publication/install_linux.py').read_text(encoding='utf-8'))
 def test_human_scene_metadata_uses_accent(self):
  lights=json.loads((ROOT/'web/scene-assets/carpushit-lights.json').read_text(encoding='utf-8'));self.assertTrue(lights['coordinates'].startswith('Schéma '))
  provenance=json.loads((ROOT/'web/scene-assets/carpushit-provenance.json').read_text(encoding='utf-8'));self.assertIn('Schéma[',provenance['transform'])
if __name__=='__main__':unittest.main()
