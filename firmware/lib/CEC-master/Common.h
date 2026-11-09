#ifndef COMMON_H__
#define COMMON_H__

#ifdef WIN32
#include <windows.h>
#include <stdio.h>
#include <assert.h>

#define ASSERT(x) assert(x)
void DbgPrint(const char* fmt, ...);

#else

#include <Arduino.h>
#define ASSERT(x) ((void)0)
void DbgPrint(const char* fmt, ...);
#ifndef NULL
#define NULL 0
#endif

#endif

#endif // COMMON_H__
