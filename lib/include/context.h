#ifndef ACERCONTROL_CONTEXT_H
#define ACERCONTROL_CONTEXT_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AcerContext AcerContext;

/*
 * Opens a connection to the Acer platform driver.
 *
 * Returns:
 *      Pointer to AcerContext on success.
 *      NULL on failure.
 */
AcerContext *acer_open(void);

/*
 * Releases all resources associated with a context.
 */
void acer_close(AcerContext *ctx);

#ifdef __cplusplus
}
#endif

#endif /* ACERCONTROL_CONTEXT_H */
