/* Pure catalogue matching; shared by the browser and Node regression tests. */
(function(global){
 'use strict';
 function normalize(value){return String(value||'').replace(/([a-z])([A-Z])/g,'$1 $2').normalize('NFKD').replace(/[\u0300-\u036f]/g,'').toLowerCase().replace(/[^a-z0-9]+/g,' ').trim();}
 function editDistance(a,b){const d=Array.from({length:a.length+1},()=>Array(b.length+1).fill(0));for(let i=0;i<=a.length;i++)d[i][0]=i;for(let j=0;j<=b.length;j++)d[0][j]=j;for(let i=1;i<=a.length;i++)for(let j=1;j<=b.length;j++){d[i][j]=Math.min(d[i-1][j]+1,d[i][j-1]+1,d[i-1][j-1]+(a[i-1]===b[j-1]?0:1));if(i>1&&j>1&&a[i-1]===b[j-2]&&a[i-2]===b[j-1])d[i][j]=Math.min(d[i][j],d[i-2][j-2]+1);}return d[a.length][b.length];}
 function termScore(needle,field){const text=normalize(field);if(!text)return 0;if(text===needle)return 180;if(text.startsWith(needle))return 130;const words=text.split(' ');if(words.includes(needle))return 115;if(words.some(w=>w.startsWith(needle)))return 90;if(text.includes(needle))return 65;
  let best=0;for(const word of words){if(needle.length>=3){let i=0,first=-1,last=-1;for(let j=0;j<word.length&&i<needle.length;j++){if(word[j]===needle[i]){if(first<0)first=j;last=j;i++;}}if(i===needle.length)best=Math.max(best,48-(last-first+1-needle.length)*3-first);const allowed=needle.length>=6?2:1;if(Math.abs(word.length-needle.length)<=allowed){const distance=editDistance(needle,word);if(distance<=allowed)best=Math.max(best,55-distance*12);}}}return best;}
 function search(rows,query='',filters={}){const terms=normalize(query).slice(0,120).split(' ').filter(Boolean).slice(0,8);const favorites=new Set(filters.favorites||[]),recent=filters.recent||[];
  return rows.filter(row=>{
   if(filters.category==='favorites'&&!favorites.has(row.key)||filters.category==='recent'&&!recent.includes(row.key))return false;
   if(filters.category&&!['all','favorites','recent'].includes(filters.category)&&row.category!==filters.category)return false;
   if(filters.domain&&row.domain!==filters.domain)return false;
   if(filters.scope==='native'&&row.extension||filters.scope==='extension'&&!row.extension)return false;return true;
  }).map(row=>{let score=0;for(const term of terms){const match=Math.max(termScore(term,row.name)*1.3,termScore(term,row.key)*1.1,termScore(term,(row.keywords||[]).join(' '))*1.15,termScore(term,row.summary),termScore(term,row.domain),termScore(term,Object.keys(row.defaults||{}).join(' '))*.85,termScore(term,row.description)*.65);if(match<=0)return null;score+=match;}return{row,score};}).filter(Boolean).sort((a,b)=>b.score-a.score||(filters.category==='recent'?recent.indexOf(a.row.key)-recent.indexOf(b.row.key):0)||(a.row.name||a.row.key).localeCompare(b.row.name||b.row.key)).map(x=>x.row);
 }
 global.SchemaCatalog={normalize,editDistance,search};
})(globalThis);
