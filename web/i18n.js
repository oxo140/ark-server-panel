'use strict';
// Only interface text is localized. User names, logs, paths, INI keys/values and
// console commands are deliberately excluded from translation.
(()=>{
const catalog=window.ARK_TRANSLATIONS, languages=['en','fr','es'];
let language=languages.includes(localStorage.getItem('ark-language'))?localStorage.getItem('ark-language'):'en';
const originals=new WeakMap(),attributes=new WeakMap();
const protectedSelector='script,style,pre,code,textarea,input,.mono,.card-map,.server-card h3,#detail-title,.sidebar-server button:first-child,.tailscale-server strong,[data-no-i18n]';
const keys=Object.keys(catalog).sort((a,b)=>b.length-a.length);
function translate(text){if(language==='fr'||!text)return text;const trimmed=text.trim();const exact=catalog[trimmed];if(exact)return text.replace(trimmed,exact[language==='en'?0:1]);
for(const [prefix,en,es]of [['Rejoindre ','Join ','Unirse a '],['Ouvrir le menu de connexion de ','Open connection menu for ','Abrir menú de conexión de ']])if(trimmed.startsWith(prefix))return text.replace(trimmed,(language==='en'?en:es)+trimmed.slice(prefix.length));
let result=text;for(const source of keys){if(source.length<4&&!/\s/.test(source))continue;if(result.includes(source))result=result.split(source).join(catalog[source][language==='en'?0:1]);}
return result.replace(/(\d+) j\b/g,'$1 d');
}
function visit(node){if(node.nodeType===3){if(!node.parentElement||node.parentElement.closest(protectedSelector))return;const last=originals.get(node);const source=last&&node.nodeValue===last.rendered?last.source:node.nodeValue;const rendered=translate(source);originals.set(node,{source,rendered});if(node.nodeValue!==rendered)node.nodeValue=rendered;return;}
if(node.nodeType!==1&&node.nodeType!==9&&node.nodeType!==11)return;
if(node.nodeType===1){for(const name of ['title','placeholder','aria-label','alt'])if(node.hasAttribute(name)){let cache=attributes.get(node);if(!cache){cache=new Map();attributes.set(node,cache);}const current=node.getAttribute(name),last=cache.get(name),source=last&&current===last.rendered?last.source:current,rendered=translate(source);cache.set(name,{source,rendered});if(current!==rendered)node.setAttribute(name,rendered);}if(node.matches(protectedSelector))return;}
for(const child of [...node.childNodes])visit(child);
}
function apply(){document.documentElement.lang=language;visit(document.body);const select=document.querySelector('#panel-language');if(select&&select.value!==language)select.value=language;}
function setLanguage(next){if(!languages.includes(next))return;language=next;localStorage.setItem('ark-language',next);apply();}
const observer=new MutationObserver(records=>{for(const record of records){if(record.type==='characterData')visit(record.target);else if(record.type==='attributes')visit(record.target);else for(const node of record.addedNodes)visit(node);}});
observer.observe(document.documentElement,{childList:true,subtree:true,characterData:true,attributes:true,attributeFilter:['title','placeholder','aria-label','alt']});
const confirm=window.confirm.bind(window);window.confirm=message=>confirm(translate(message));
window.ArkI18n={translate,setLanguage,apply,language:()=>language,locale:()=>({en:'en-US',fr:'fr-FR',es:'es-ES'})[language]};apply();
})();
