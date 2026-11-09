#include <stdlib.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include "Common.h"

#ifdef WIN32

static char FormatBuffer[4096]; 
static CRITICAL_SECTION CriticalSection;
static bool CriticalSectionInitialized = false;

void DbgPrint(const char* fmt, ...)
{
	va_list args;

	if (!CriticalSectionInitialized)
	{
		InitializeCriticalSection(&CriticalSection);
		CriticalSectionInitialized = true;
	}

	EnterCriticalSection(&CriticalSection);

	va_start(args, fmt);
	vsprintf_s(FormatBuffer, 4096, fmt, args);
	OutputDebugStringA(FormatBuffer);

	LeaveCriticalSection(&CriticalSection);
}

#else

#include <Arduino.h>

void DbgPrint(const char* fmt, ...)
{
	char FormatBuffer[128]; 
	va_list args;
	va_start(args, fmt);
	vsnprintf(FormatBuffer, sizeof(FormatBuffer), fmt, args);
	va_end(args);
	Serial.print(FormatBuffer);
}

#endif
