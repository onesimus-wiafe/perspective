#  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
#  ┃ ██████ ██████ ██████       █      █      █      █      █ █▄  ▀███ █       ┃
#  ┃ ▄▄▄▄▄█ █▄▄▄▄▄ ▄▄▄▄▄█  ▀▀▀▀▀█▀▀▀▀▀ █ ▀▀▀▀▀█ ████████▌▐███ ███▄  ▀█ █ ▀▀▀▀▀ ┃
#  ┃ █▀▀▀▀▀ █▀▀▀▀▀ █▀██▀▀ ▄▄▄▄▄ █ ▄▄▄▄▄█ ▄▄▄▄▄█ ████████▌▐███ █████▄   █ ▄▄▄▄▄ ┃
#  ┃ █      ██████ █  ▀█▄       █ ██████      █      ███▌▐███ ███████▄ █       ┃
#  ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
#  ┃ Copyright (c) 2017, the Perspective Authors.                              ┃
#  ┃ ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ ┃
#  ┃ This file is part of the Perspective library, distributed under the terms ┃
#  ┃ of the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). ┃
#  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

import asyncio
import os
import os.path
import logging
import uvicorn

from fastapi import FastAPI, WebSocket
from fastapi.middleware.cors import CORSMiddleware
from starlette.responses import FileResponse
from starlette.staticfiles import StaticFiles
from threading import Thread

from perspective import LocalPerspective
from perspective.handlers.starlette import PerspectiveStarletteHandler


here = os.path.abspath(os.path.dirname(__file__))
file_path = os.path.join(
    here, "..", "..", "node_modules", "superstore-arrow", "superstore.lz4.arrow"
)


def static_nodemodules_handler(rest_of_path):
    if rest_of_path.startswith("@finos"):
        return FileResponse("../../node_modules/{}".format(rest_of_path))
    return FileResponse("../../node_modules/@finos/{}".format(rest_of_path))


def make_app():
    psp = LocalPerspective()

    async def websocket_handler(websocket: WebSocket):
        handler = PerspectiveStarletteHandler(perspective=psp, websocket=websocket)
        await handler.run()

    async def init_table():
        table = psp.client.table({"x": "integer"}, name="data_source_one")
        size = 10
        for i in range(size):
            rows = [*range(i * size, (i + 1) * size)]
            table.update({'x': rows})
            await asyncio.sleep(5)

    Thread(target=asyncio.run, args=(init_table(), ), daemon=True).start()

    # static_html_files = StaticFiles(directory="../python-tornado", html=True)
    static_html_files = StaticFiles(directory="../python-tornado", html=True)

    app = FastAPI()
    app.add_api_websocket_route("/websocket", websocket_handler)
    app.get("/node_modules/{rest_of_path:path}")(static_nodemodules_handler)
    app.mount("/", static_html_files)

    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )
    return app


if __name__ == "__main__":
    app = make_app()
    logging.critical("Listening on http://localhost:8082")
    uvicorn.run(app, host="0.0.0.0", port=8082)
