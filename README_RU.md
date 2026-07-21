# Проект RTCP (RTC Proxy) (прошлое название TURN proxy)

![GitHub License](https://img.shields.io/github/license/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=gplv3&logoColor=FFFFFF)
![GitHub repo size](https://img.shields.io/github/repo-size/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=github&logoColor=FFFFFF)
![GitHub top language](https://img.shields.io/github/languages/top/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=rust&color=FF8000&logoColor=FFFFFF)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/Urtyom-Alyanov/turn-proxy/check.yml?style=for-the-badge&label=Checks)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/Urtyom-Alyanov/turn-proxy/build.yml?style=for-the-badge&label=Builds)
![Last Commit](https://img.shields.io/github/last-commit/Urtyom-Alyanov/turn-proxy?style=for-the-badge&logo=git&logoColor=FFFFFF)

[README on English](./README.md)

Данный проект реализует протокол WebRTC для того, чтобы через этот протокол обмениваться **любой информацией**. Это является развитием проекта TURN proxy, что просто шифровал с помощью DTLS UPD пакеты и обворачивал их TURN заголовком.

Данный подход уже обнаруживается системами ВК (как основного поставщика). Также прошлый проект хоть и был неплохо написан, но всё равно упёрся в свой архитектурный "лимит". Это буквально мой первый проект на Rust, сейчас у меня навки лучше, поэтому я вижу, что архитектуру лучше пересмотреть.

> [!IMPORTANT]
> Данный проект пишется мною в первую очередь для изучения сетевого и "низкоуровневого" программирования.
> Следовательно он не имеет основного use-case и может использоваться в любых ситуациях.

## Связанные проекты

- [olcrtc](https://github.com/openlibrecommunity/olcrtc) - данный проект во многом экспериментальный и много будет взято оттуда. По сути тут реализовано то, что я хотел реализовать ещё в прошлом проекте. Лицензируется под WTFPL, написан на Go. Рекомендую использовать его в качестве замены устаревшим vk-turn-proxy и TURN proxy
- [Good TURN (vk-turn-proxy)](https://github.com/cacggghp/vk-turn-proxy) - чутка подзаброшенный проект, по сути первый популярный проект в этой стране, что популяризовал данный метод. Тоже вдохновитель. Написан на GO, лицензируется под GPL-3.0. Также там расположены все проекты что тоже частично повлияли на сей проект.

## Почему Rust

У Rust есть некоторые преимущества:

- Zero-cost на, почти что, всё. Приложение за неимением GC может позволить себя упаковать в swap, отчего в самой ОЗУ будет заниматься 0 байт. 
- Многопоточность "без страха", то есть сами механизмы языка (владение и заимствование) позволяют использовать корутины или потоки ОС безопасно, то есть без всяких гонок.
- Производительность бинарника, так как Rust построен поверх LLVM, у которого очень хороший оптимизатор, а также правила самого Rust дают больше информации и "уверенности" самому оптимизатору LLVM, то бинарник в следствие этого получается очень оптимизированным и шустрым.

Но есть и недостаток главный перед Go: незрелость библиотек самого WebRTC.

## Архитектура проекта

Проект ради простоты разработки разделён на несколько крейтов и проектов.

В `/providers/` лежат поставщики серверов, через которых это соединение и идёт.
В `/transports/` лежат, скажем так, "фильтры", что преобразуют ваш трафик. Тут вся шифровка, все приколы и прочее.
В `/lib/` лежит по сути сам "голый" проект поставляемый как библиотека для всяких клиентов и прочих проектов.
В `/client/` лежит сам CLI клиент, что сочетает в себе и сервер и клиент, и его конфиг.

### Лицензирование

Сама библиотека (то есть `/lib/`) и связанные с ней крейты (`/providers/`, `/transports/`) лицензируется под [MPL-2.0 (Mozilla Public License 2.0)](./lib/LICENSE). Клиент под [GNU AGPL-3.0 (GNU Affero General Public License 3.0)](./client/app/LICENSE).

Сделано ради соблюдения прав на модификацию, использования в любых целях и прочих свобод вне зависимости от форка.


