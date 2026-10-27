#!/usr/bin/python3
"""Patch Art-Net input -> universe 0 -> the selected Open DMX FTDI output."""
import json,os,pathlib,sys,time
ARTNET=2
FTDIDMX=13

def make_plan(devices,ftdi_uid=''):
    artnet=[d for d in devices if d['plugin_id']==ARTNET and d['input_ports']]
    if len(artnet)!=1:raise ValueError('Expected exactly one Art-Net input device')
    ftdi=[d for d in devices if d['plugin_id']==FTDIDMX and d['output_ports'] and (not ftdi_uid or d['id']==ftdi_uid)]
    if len(ftdi)>1:raise ValueError('Multiple FTDI adapters; set SCHEMA_FTDI_UID before bridging')
    selected=[(artnet[0],False)] + ([(ftdi[0],True)] if ftdi else [])
    patches=[]
    for device,is_output in selected:
        ports=device['output_ports'] if is_output else device['input_ports']
        port=min(ports,key=lambda p:p['id'])
        if port['universe']!=0:patches.append({'alias':device['alias'],'port':port['id'],'is_output':is_output,'uid':device['id']})
    return {'patches':patches,'usb_found':bool(ftdi),'ftdi_uid':ftdi[0]['id'] if ftdi else None,'artnet_uid':artnet[0]['id']}

def write_status(result):
    result['updated_utc']=int(time.time())
    target=pathlib.Path('/run/schema-ola/status.json');target.parent.mkdir(exist_ok=True)
    temporary=target.with_name('.status-'+str(os.getpid())+'.tmp');temporary.write_text(json.dumps(result,indent=2)+chr(10));temporary.replace(target)

def main():
    from ola.ClientWrapper import ClientWrapper
    from ola.OlaClient import OlaClient
    wrapper=ClientWrapper();client=wrapper.Client();errors=[];result={}
    def stop_error(message):errors.append(str(message));wrapper.Stop()
    def devices_ready(status,devices):
        if not status.Succeeded():return stop_error(status)
        def port(p):return {'id':p.id,'universe':p.universe}
        try:
            result.update(make_plan([{'plugin_id':d.plugin_id,'alias':d.alias,'id':d.id,'input_ports':[port(p) for p in d.input_ports],'output_ports':[port(p) for p in d.output_ports]} for d in devices],os.environ.get('SCHEMA_FTDI_UID','')))
        except ValueError as e:return stop_error(e)
        pending=len(result['patches'])
        if not pending:return wrapper.Stop()
        def patched(status):
            nonlocal pending
            if not status.Succeeded():errors.append(str(status))
            pending-=1
            if pending==0:wrapper.Stop()
        for p in result['patches']:client.PatchPort(p['alias'],p['port'],p['is_output'],OlaClient.PATCH,0,patched)
    client.FetchDevices(devices_ready);wrapper.AddEvent(5000,lambda:stop_error('OLA reconciliation timed out'));wrapper.Run()
    if errors:raise RuntimeError('; '.join(errors))
    result['state']='ready' if result['usb_found'] else 'waiting_for_open_dmx_usb'
    write_status(result)
    print(json.dumps(result))
if __name__=='__main__':
    try:main()
    except Exception as error:
        try:write_status({'state':'error','error':str(error)})
        except OSError:pass
        print(str(error),file=sys.stderr);sys.exit(1)
