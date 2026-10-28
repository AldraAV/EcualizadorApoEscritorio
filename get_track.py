import asyncio

async def get_media_info():
    try:
        from winsdk.windows.media.control import GlobalSystemMediaTransportControlsSessionManager
        
        manager = await GlobalSystemMediaTransportControlsSessionManager.request_async()
        session = manager.get_current_session()
        
        if not session:
            print("No se encontro ninguna sesion multimedia activa.")
            return

        info = await session.try_get_media_properties_async()
        print(f"PISTA_ACTUAL: {info.artist} - {info.title}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    asyncio.run(get_media_info())
