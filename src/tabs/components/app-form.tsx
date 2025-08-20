import { zodResolver } from "@hookform/resolvers/zod";
import React from "react";
import { Button } from "@/components/ui/button";
import { useAppStore } from "@/store";
import { z } from "zod";
import { useForm } from "react-hook-form";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";

const appFormSchema = z.object({
  name: z.string().min(2).max(100),
  ids: z.array(z.string()),
});

type AppFormData = z.infer<typeof appFormSchema>;

const AppForm = React.memo(() => {
  const { fetchApps, apps } = useAppStore();
  const form = useForm<AppFormData>({
    resolver: zodResolver(appFormSchema),
  });

  const onSubmit = async (data: AppFormData) => {
    const toastId = toast.loading("Saving backup...");

    try {
      const result = await invoke<string>("save_config", {
        name: data.name,
        appIds: data.ids,
      });
      toast.success(result, {
        id: toastId,
        description: "Backup saved successfully!",
      });
    } catch (e) {
      toast.error("Failed to save backup!", { id: toastId });
      console.error(e);
    }
  };

  const fetchAppsCallback = React.useCallback(() => {
    fetchApps();
  }, [fetchApps]);

  React.useEffect(() => {
    fetchAppsCallback();
  }, [fetchAppsCallback]);

  return (
    <div>
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-8">
          <FormField
            control={form.control}
            name="ids"
            rules={{ required: true }}
            render={() => (
              <FormItem>
                <div className="mb-4">
                  <FormLabel>Applications</FormLabel>
                  <FormDescription>
                    Select the applications you want to backup.
                  </FormDescription>
                </div>
                {apps
                  .filter((item) => item.is_installed)
                  .map((item) => (
                    <FormField
                      key={item.id}
                      control={form.control}
                      name="ids"
                      render={({ field }) => {
                        return (
                          <FormItem
                            key={item.id}
                            className="flex flex-row items-center gap-2"
                          >
                            <FormControl>
                              <Checkbox
                                checked={field.value?.includes(item.id)}
                                onCheckedChange={(checked) => {
                                  return checked
                                    ? field.onChange([...(field.value || []), item.id])
                                    : field.onChange(
                                        field.value?.filter(
                                          (value) => value !== item.id
                                        )
                                      );
                                }}
                              />
                            </FormControl>
                            <FormLabel className="text-sm font-normal">
                              {item.name}
                            </FormLabel>
                          </FormItem>
                        );
                      }}
                    />
                  ))}
              </FormItem>
            )}
          />
          <FormField
            control={form.control}
            name="name"
            rules={{ required: true }}
            render={({ field }) => (
              <FormItem>
                <FormLabel>Backup Name</FormLabel>
                <FormControl>
                  <Input placeholder="e.g., My-Laptop-Setup" {...field} />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
          <Button type="submit">Submit</Button>
        </form>
      </Form>
    </div>
  );
});

export default AppForm;
