/* TEMPLATE GENERATED TESTCASE FILE
Filename: CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22a.c
Label Definition File: CWE427_Uncontrolled_Search_Path_Element.label.xml
Template File: sources-sink-22a.tmpl.c
*/
/*
 * @description
 * CWE: 427 Uncontrolled Search Path Element
 * BadSource: connect_socket Read data using a connect socket (client side)
 * GoodSource: Use a hardcoded path
 * Sink:
 *    BadSink : Set the environment variable
 * Flow Variant: 22 Control flow: Flow controlled by value of a global variable. Sink functions are in a separate file from sources.
 *
 * */

#include "std_testcase.h"

#include <wchar.h>
#ifdef _WIN32
#define NEW_PATH L"%SystemRoot%\\system32"
#define PUTENV _wputenv
#else
#define NEW_PATH L"/bin"
#define PUTENV putenv
#endif

#ifndef OMITBAD

/* The global variable below is used to drive control flow in the source function */
int CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_badGlobal = 0;

wchar_t * CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_badSource(wchar_t * data);

void CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_bad()
{
    wchar_t * data;
    wchar_t dataBuffer[250] = L"PATH=";
    data = dataBuffer;
    CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_badGlobal = 1; /* true */
    data = CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_badSource(data);
    /* POTENTIAL FLAW: Set a new environment variable with a path that is possibly insecure */
    PUTENV(data);
}

#endif /* OMITBAD */

#ifndef OMITGOOD

/* The global variables below are used to drive control flow in the source functions. */
int CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B1Global = 0;
int CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B2Global = 0;

/* goodG2B1() - use goodsource and badsink by setting the static variable to false instead of true */
wchar_t * CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B1Source(wchar_t * data);

static void goodG2B1()
{
    wchar_t * data;
    wchar_t dataBuffer[250] = L"PATH=";
    data = dataBuffer;
    CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B1Global = 0; /* false */
    data = CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B1Source(data);
    /* POTENTIAL FLAW: Set a new environment variable with a path that is possibly insecure */
    PUTENV(data);
}

/* goodG2B2() - use goodsource and badsink by reversing the blocks in the if in the source function */
wchar_t * CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B2Source(wchar_t * data);

static void goodG2B2()
{
    wchar_t * data;
    wchar_t dataBuffer[250] = L"PATH=";
    data = dataBuffer;
    CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B2Global = 1; /* true */
    data = CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_goodG2B2Source(data);
    /* POTENTIAL FLAW: Set a new environment variable with a path that is possibly insecure */
    PUTENV(data);
}

void CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_good()
{
    goodG2B1();
    goodG2B2();
}

#endif /* OMITGOOD */

/* Below is the main(). It is only used when building this testcase on
 * its own for testing or for building a binary to use in testing binary
 * analysis tools. It is not used when compiling all the testcases as one
 * application, which is how source code analysis tools are tested.
 */

#ifdef INCLUDEMAIN

int main(int argc, char * argv[])
{
    /* seed randomness */
    srand( (unsigned)time(NULL) );
#ifndef OMITGOOD
    printLine("Calling good()...");
    CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_good();
    printLine("Finished good()");
#endif /* OMITGOOD */
#ifndef OMITBAD
    printLine("Calling bad()...");
    CWE427_Uncontrolled_Search_Path_Element__wchar_t_connect_socket_22_bad();
    printLine("Finished bad()");
#endif /* OMITBAD */
    return 0;
}

#endif